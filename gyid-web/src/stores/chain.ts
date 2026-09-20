// 本地面包屑链 store（按 pubkey 隔离，持久化到 localStorage）。
//
// 每条记录三组 key：
// - gyid.chain.<pubkey>     BreadcrumbJs[]（已签名面包屑，CBOR 可还原）
// - gyid.coords.<pubkey>    CoordsPoint[]（采集时的原始 GPS 坐标，仅供本地地图展示；
//                           Breadcrumb 本身只含 H3 cell，不含原始经纬度）
// - gyid.uploaded.<pubkey>  number（已成功上传到 Verifier 的条数，剩余的待同步）

import { createStore } from "solid-js/store";
import { loadJson, saveJson } from "../lib/storage";
import { loadWasm, type BreadcrumbJs } from "../lib/wasm";

/** 单次采集的原始坐标（地图轨迹用）。 */
export interface CoordsPoint {
  index: number;
  lat: number;
  lng: number;
  ts: number;
}

interface ChainState {
  /** 当前载入的链属于哪个 pubkey；null 表示尚未 loadChain */
  pubkey: string | null;
  crumbs: BreadcrumbJs[];
  coords: CoordsPoint[];
  uploadedCount: number;
}

const [state, setState] = createStore<ChainState>({
  pubkey: null,
  crumbs: [],
  coords: [],
  uploadedCount: 0,
});

const chainKey = (pub: string) => `gyid.chain.${pub}`;
const coordsKey = (pub: string) => `gyid.coords.${pub}`;
const uploadedKey = (pub: string) => `gyid.uploaded.${pub}`;

/** POST /v1/evidence 单批上限（draft-04 §5；服务端按批处理）。 */
export const MAX_EVIDENCE_BATCH = 300;

/** store 状态（只读） */
export const chainStore = state;

/** 载入某 pubkey 的本地链（切换账户时调用）。重复载入同一 pubkey 无副作用。 */
export function loadChain(pubkey: string): void {
  if (state.pubkey === pubkey) return;
  setState({
    pubkey,
    crumbs: loadJson<BreadcrumbJs[]>(chainKey(pubkey), []),
    coords: loadJson<CoordsPoint[]>(coordsKey(pubkey), []),
    uploadedCount: loadJson<number>(uploadedKey(pubkey), 0),
  });
}

export interface CollectArgs {
  seedHex: string;
  lat: number;
  lng: number;
  /** H3 分辨率 7..=10 */
  resolution: number;
  /** §4.2 探索会话（允许 5..15 分钟间隔） */
  exploration: boolean;
  /** Unix 秒；默认当前时间 */
  timestamp?: number;
}

/**
 * 采集 + 签名下一条面包屑并追加到本地链。
 *
 * index / prev_hash 由链尾自动推出：
 * - 空链：index=0, prev_hash=null（创世）
 * - 非空：index=last.index+1, prev_hash=last.block_hash
 */
export async function collectNext(args: CollectArgs): Promise<BreadcrumbJs> {
  const wasm = await loadWasm();
  if (!state.pubkey) throw new Error("chain not loaded; call loadChain(pubkey) first");

  const ts = args.timestamp ?? Math.floor(Date.now() / 1000);
  const last = state.crumbs[state.crumbs.length - 1];

  const crumb = wasm.collect_breadcrumb(
    args.seedHex,
    args.lat,
    args.lng,
    args.resolution,
    BigInt(ts),
    last ? BigInt(last.index + 1) : null,
    last ? last.block_hash_hex : null,
    args.exploration,
  );

  const crumbs = [...state.crumbs, crumb];
  const coords = [
    ...state.coords,
    { index: crumb.index, lat: args.lat, lng: args.lng, ts },
  ];
  setState({ crumbs, coords });
  saveJson(chainKey(state.pubkey), crumbs);
  saveJson(coordsKey(state.pubkey), coords);
  return crumb;
}

// ─────────────────── 手动导入（GPX / 照片轨迹，W6）───────────────────

/** 协议链规则常量（与 trip-core::ChainRules::default 逐项对齐）。 */
export const CHAIN_MIN_INTERVAL_SECS = 900; // §4.1 默认采集间隔
export const CHAIN_HARD_MIN_INTERVAL_SECS = 300; // §4.2 硬下限（探索会话）
export const CHAIN_MAX_PER_CELL = 10; // 单 cell 累计上限

/** 导入时被丢弃的原因。 */
export type ImportSkipReason =
  | "same-cell" // 与上一条落同一 H3 cell（§4.1 连续去重）
  | "cell-cap" // 该 cell 累计已达上限
  | "too-soon" // 与上一条间隔不足（< 硬下限，或未开探索会话而 < 900s）
  | "time-backwards"; // 时间早于链尾（导入的轨迹早于本地已采集记录）

export interface ImportSkip {
  lat: number;
  lng: number;
  ts: number;
  reason: ImportSkipReason;
}

export interface ImportResult {
  /** 成功签名并追加的出数 */
  added: number;
  /** 被链规则丢弃的点 */
  skipped: ImportSkip[];
}

export interface ImportArgs {
  seedHex: string;
  /** 候选轨迹点（内部按 ts 升序排序） */
  points: { lat: number; lng: number; ts: number }[];
  /** H3 分辨率 7..=10 */
  resolution: number;
  /** §4.2 探索会话：允许 5..15 分钟间隔 */
  exploration: boolean;
  /** 进度回调（已处理 / 总数） */
  onProgress?: (done: number, total: number) => void;
}

/** 预览结果：只统计，不签名、不落盘。 */
export interface ImportPreview {
  /** 会被接受的面包屑数 */
  accepted: number;
  skipped: ImportSkip[];
  /** 候选点总数 */
  total: number;
}

/** 通过链规则、等待签名的候选（index 已定，prev_hash 由签名循环续接）。 */
interface PendingCrumb {
  lat: number;
  lng: number;
  ts: number;
  index: number;
}

interface ImportPlan {
  points: { lat: number; lng: number; ts: number }[];
  pending: PendingCrumb[];
  skipped: ImportSkip[];
}

/**
 * §4 链规则过滤（纯函数，不依赖 WASM）：把候选点划分为「可签名」与「丢弃」。
 *
 * 规则顺序与 `trip-core::ChainRules::verify` 一致；`lastTs`/`lastCell` 起点为
 * 链尾，因此导入必须与已有链自然接续（更早的点一律丢弃）。
 */
function planImport(
  points: { lat: number; lng: number; ts: number }[],
  cells: string[],
  exploration: boolean,
  tail: BreadcrumbJs | undefined,
  cellCounts: Map<string, number>,
): ImportPlan {
  const minInterval = exploration
    ? CHAIN_HARD_MIN_INTERVAL_SECS
    : CHAIN_MIN_INTERVAL_SECS;

  let index = tail ? tail.index + 1 : 0;
  let lastTs: number | null = tail ? tail.timestamp : null;
  let lastCell: string | null = tail ? tail.h3_cell_hex : null;

  const pending: PendingCrumb[] = [];
  const skipped: ImportSkip[] = [];

  for (let i = 0; i < points.length; i++) {
    const p = points[i];
    const cell = cells[i];

    if (lastCell !== null && cell === lastCell) {
      skipped.push({ ...p, reason: "same-cell" });
      continue;
    }
    if (lastTs !== null) {
      const delta = p.ts - lastTs;
      if (delta < 0) {
        skipped.push({ ...p, reason: "time-backwards" });
        continue;
      }
      if (delta < minInterval) {
        skipped.push({ ...p, reason: "too-soon" });
        continue;
      }
    }
    if ((cellCounts.get(cell) ?? 0) >= CHAIN_MAX_PER_CELL) {
      skipped.push({ ...p, reason: "cell-cap" });
      continue;
    }

    pending.push({ lat: p.lat, lng: p.lng, ts: p.ts, index });
    cellCounts.set(cell, (cellCounts.get(cell) ?? 0) + 1);
    index += 1;
    lastTs = p.ts;
    lastCell = cell;
  }

  return { points, pending, skipped };
}

/** 量化候选点并跑链规则，得到统一计划（预览与导入共用）。 */
async function buildPlan(
  points: { lat: number; lng: number; ts: number }[],
  resolution: number,
  exploration: boolean,
): Promise<ImportPlan> {
  const wasm = await loadWasm();
  const sorted = [...points].sort((a, b) => a.ts - b.ts);
  const cells: string[] = [];
  for (const p of sorted) {
    cells.push(wasm.h3_to_cell_hex(p.lat, p.lng, resolution));
  }

  // 单 cell 计数需把链上已有面包屑一并计入
  const cellCounts = new Map<string, number>();
  for (const c of state.crumbs) {
    cellCounts.set(c.h3_cell_hex, (cellCounts.get(c.h3_cell_hex) ?? 0) + 1);
  }

  return planImport(
    sorted,
    cells,
    exploration,
    state.crumbs[state.crumbs.length - 1],
    cellCounts,
  );
}

/**
 * 预演导入：只计算「会接受多少 / 会丢弃哪些」，不签名、不落盘。
 * UI 用它先把丢弃原因展示给用户。
 */
export async function previewImport(args: {
  points: { lat: number; lng: number; ts: number }[];
  resolution: number;
  exploration: boolean;
}): Promise<ImportPreview> {
  if (!state.pubkey) {
    throw new Error("chain not loaded; call loadChain(pubkey) first");
  }
  const plan = await buildPlan(args.points, args.resolution, args.exploration);
  return {
    accepted: plan.pending.length,
    skipped: plan.skipped,
    total: plan.points.length,
  };
}

/**
 * 把手动导入的轨迹点批量转成面包屑并追加到本地链。
 *
 * 与实时采集的区别：时间戳来自文件（不是「现在」），因此**严格**执行 §4 链
 * 规则——不满足的点丢弃并计入 `skipped`，绝不改写时间戳：
 * - 连续落在同一 H3 cell → 丢弃（§4.1）；
 * - 与前一条间隔 < 300s → 丢弃（§4.2 硬下限）；
 * - 间隔 < 900s 且未开启探索会话 → 丢弃（§4.1 默认）；
 * - 单 cell 累计 > 10 → 丢弃（防静止耕作）；
 * - 早于链尾时间 → 丢弃（导入旧轨迹前请先清空本地链）。
 *
 * index / prev_hash 由链尾自动续接（逻辑同 [`collectNext`]）；末尾一次性落盘。
 */
export async function importCrumbs(args: ImportArgs): Promise<ImportResult> {
  if (!state.pubkey) {
    throw new Error("chain not loaded; call loadChain(pubkey) first");
  }
  const wasm = await loadWasm();
  const plan = await buildPlan(args.points, args.resolution, args.exploration);

  const tail = state.crumbs[state.crumbs.length - 1];
  let prevHash: string | null = tail ? tail.block_hash_hex : null;

  const addedCrumbs: BreadcrumbJs[] = [];
  const addedCoords: CoordsPoint[] = [];

  for (let k = 0; k < plan.pending.length; k++) {
    const p = plan.pending[k];
    args.onProgress?.(k + 1, plan.pending.length);

    // 空链首条必须是 index=0 + prev_hash=null（wasm 要求两者同时缺省）
    const hasPrev = tail !== undefined || k > 0;
    const crumb = wasm.collect_breadcrumb(
      args.seedHex,
      p.lat,
      p.lng,
      args.resolution,
      BigInt(p.ts),
      hasPrev ? BigInt(p.index) : null,
      hasPrev ? prevHash : null,
      args.exploration,
    );

    addedCrumbs.push(crumb);
    addedCoords.push({ index: crumb.index, lat: p.lat, lng: p.lng, ts: p.ts });
    prevHash = crumb.block_hash_hex;
  }

  if (addedCrumbs.length > 0) {
    const crumbs = [...state.crumbs, ...addedCrumbs];
    const coords = [...state.coords, ...addedCoords];
    setState({ crumbs, coords });
    saveJson(chainKey(state.pubkey), crumbs);
    saveJson(coordsKey(state.pubkey), coords);
  }

  return { added: addedCrumbs.length, skipped: plan.skipped };
}

/**
 * 上传所有未同步面包屑（自动按 MAX_EVIDENCE_BATCH 分批，顺序提交）。
 * 任一批失败即抛出，已成功推进的 uploadedCount 保留，下次调用从断点续传。
 * 返回本次新上传的条数。
 */
export async function uploadPending(
  uploadFn: (cborHex: string) => Promise<{ stored: number }>,
): Promise<number> {
  const wasm = await loadWasm();
  let uploaded = 0;
  while (state.uploadedCount + uploaded < state.crumbs.length) {
    const start = state.uploadedCount + uploaded;
    const batch = state.crumbs.slice(start, start + MAX_EVIDENCE_BATCH);
    const cborHex = wasm.chain_to_cbor_hex(batch);
    await uploadFn(cborHex);
    uploaded += batch.length;
  }
  const newCount = state.uploadedCount + uploaded;
  setState({ uploadedCount: newCount });
  if (state.pubkey) saveJson(uploadedKey(state.pubkey), newCount);
  return uploaded;
}

/** 全链自检（§4.1/§4.2/§4.3 时间/签名/衔接规则）。 */
export async function verifyLocalChain(): Promise<boolean> {
  const wasm = await loadWasm();
  return wasm.verify_chain(state.crumbs);
}

/** 清空本地链（需二次确认的 UI 调用）。 */
export function resetChain(): void {
  if (!state.pubkey) return;
  setState({ crumbs: [], coords: [], uploadedCount: 0 });
  saveJson(chainKey(state.pubkey), []);
  saveJson(coordsKey(state.pubkey), []);
  saveJson(uploadedKey(state.pubkey), 0);
}
