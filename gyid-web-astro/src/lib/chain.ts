/**
 * 本地面包屑链（按 pubkey 隔离，持久化到 localStorage）。
 *
 * key 与 gyid-web 完全一致，便于两个控制台互换：
 * - `gyid.chain.<pubkey>`     BreadcrumbJs[]（已签名面包屑）
 * - `gyid.coords.<pubkey>`    采集时的原始坐标（仅供本地地图展示；协议消息只含 H3 cell）
 * - `gyid.uploaded.<pubkey>`  已成功上传的条数
 */
import { loadWasm, nowSecs, type BreadcrumbJs } from './wasm';
import { loadJson, saveJson } from './storage';

export interface CoordsPoint {
  index: number;
  lat: number;
  lng: number;
  ts: number;
}

interface ChainState {
  pubkey: string | null;
  crumbs: BreadcrumbJs[];
  coords: CoordsPoint[];
  uploadedCount: number;
}

const state: ChainState = { pubkey: null, crumbs: [], coords: [], uploadedCount: 0 };

const chainKey = (pub: string) => `gyid.chain.${pub}`;
const coordsKey = (pub: string) => `gyid.coords.${pub}`;
const uploadedKey = (pub: string) => `gyid.uploaded.${pub}`;

/** POST /v1/evidence 单批上限（draft-04 §5）。 */
export const MAX_EVIDENCE_BATCH = 300;

/** 协议链规则常量（与 trip-core::ChainRules::default 逐项对齐）。 */
export const CHAIN_MIN_INTERVAL_SECS = 900;
export const CHAIN_HARD_MIN_INTERVAL_SECS = 300;
export const CHAIN_MAX_PER_CELL = 10;

export type ImportSkipReason = 'same-cell' | 'cell-cap' | 'too-soon' | 'time-backwards';

export interface ImportSkip {
  lat: number;
  lng: number;
  ts: number;
  reason: ImportSkipReason;
}

export interface ImportPreview {
  accepted: number;
  skipped: ImportSkip[];
  total: number;
}

export interface ImportResult {
  added: number;
  skipped: ImportSkip[];
}

export function chainState(): Readonly<ChainState> {
  return state;
}

/** 载入某 pubkey 的本地链；重复载入同一 pubkey 无副作用。 */
export function loadChain(pubkey: string): void {
  if (state.pubkey === pubkey) return;
  state.pubkey = pubkey;
  state.crumbs = loadJson<BreadcrumbJs[]>(chainKey(pubkey), []);
  state.coords = loadJson<CoordsPoint[]>(coordsKey(pubkey), []);
  state.uploadedCount = loadJson<number>(uploadedKey(pubkey), 0);
}

/** 采集 + 签名下一条面包屑并追加到本地链。 */
export async function collectNext(args: {
  seedHex: string;
  lat: number;
  lng: number;
  resolution: number;
  exploration: boolean;
  timestamp?: number;
}): Promise<BreadcrumbJs> {
  const wasm = await loadWasm();
  if (!state.pubkey) throw new Error('chain not loaded');
  const ts = args.timestamp ?? nowSecs();
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
  state.crumbs = [...state.crumbs, crumb];
  state.coords = [...state.coords, { index: crumb.index, lat: args.lat, lng: args.lng, ts }];
  saveJson(chainKey(state.pubkey), state.crumbs);
  saveJson(coordsKey(state.pubkey), state.coords);
  return crumb;
}

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
 * 规则顺序与 `trip-core::ChainRules::verify` 一致。
 */
function planImport(
  points: { lat: number; lng: number; ts: number }[],
  cells: string[],
  exploration: boolean,
  tail: BreadcrumbJs | undefined,
  cellCounts: Map<string, number>,
): ImportPlan {
  const minInterval = exploration ? CHAIN_HARD_MIN_INTERVAL_SECS : CHAIN_MIN_INTERVAL_SECS;
  let index = tail ? tail.index + 1 : 0;
  let lastTs: number | null = tail ? tail.timestamp : null;
  let lastCell: string | null = tail ? tail.h3_cell_hex : null;

  const pending: PendingCrumb[] = [];
  const skipped: ImportSkip[] = [];

  for (let i = 0; i < points.length; i++) {
    const p = points[i]!;
    const cell = cells[i]!;
    if (lastCell !== null && cell === lastCell) {
      skipped.push({ ...p, reason: 'same-cell' });
      continue;
    }
    if (lastTs !== null) {
      const delta = p.ts - lastTs;
      if (delta < 0) {
        skipped.push({ ...p, reason: 'time-backwards' });
        continue;
      }
      if (delta < minInterval) {
        skipped.push({ ...p, reason: 'too-soon' });
        continue;
      }
    }
    if ((cellCounts.get(cell) ?? 0) >= CHAIN_MAX_PER_CELL) {
      skipped.push({ ...p, reason: 'cell-cap' });
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

async function buildPlan(
  points: { lat: number; lng: number; ts: number }[],
  resolution: number,
  exploration: boolean,
): Promise<ImportPlan> {
  if (!state.pubkey) throw new Error('chain not loaded');
  const wasm = await loadWasm();
  const sorted = [...points].sort((a, b) => a.ts - b.ts);
  const cells = sorted.map((p) => wasm.h3_to_cell_hex(p.lat, p.lng, resolution));

  const cellCounts = new Map<string, number>();
  for (const c of state.crumbs) {
    cellCounts.set(c.h3_cell_hex, (cellCounts.get(c.h3_cell_hex) ?? 0) + 1);
  }
  return planImport(sorted, cells, exploration, state.crumbs[state.crumbs.length - 1], cellCounts);
}

/** 预演导入：只统计，不签名、不落盘。 */
export async function previewImport(args: {
  points: { lat: number; lng: number; ts: number }[];
  resolution: number;
  exploration: boolean;
}): Promise<ImportPreview> {
  const plan = await buildPlan(args.points, args.resolution, args.exploration);
  return { accepted: plan.pending.length, skipped: plan.skipped, total: plan.points.length };
}

/**
 * 批量导入轨迹点 → 面包屑并追加到本地链。
 * 时间戳严格来自文件，不满足链规则的点丢弃（绝不改写时间戳）。
 */
export async function importCrumbs(args: {
  seedHex: string;
  points: { lat: number; lng: number; ts: number }[];
  resolution: number;
  exploration: boolean;
  onProgress?: (done: number, total: number) => void;
}): Promise<ImportResult> {
  const wasm = await loadWasm();
  const plan = await buildPlan(args.points, args.resolution, args.exploration);
  const tail = state.crumbs[state.crumbs.length - 1];
  let prevHash: string | null = tail ? tail.block_hash_hex : null;

  const addedCrumbs: BreadcrumbJs[] = [];
  const addedCoords: CoordsPoint[] = [];

  for (let k = 0; k < plan.pending.length; k++) {
    const p = plan.pending[k]!;
    args.onProgress?.(k + 1, plan.pending.length);
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
    state.crumbs = [...state.crumbs, ...addedCrumbs];
    state.coords = [...state.coords, ...addedCoords];
    saveJson(chainKey(state.pubkey!), state.crumbs);
    saveJson(coordsKey(state.pubkey!), state.coords);
  }
  return { added: addedCrumbs.length, skipped: plan.skipped };
}

/**
 * 上传所有未同步面包屑（按 MAX_EVIDENCE_BATCH 分批，顺序提交）。
 * 任一批失败即抛出；已成功推进的 uploadedCount 保留，下次从断点续传。
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
  state.uploadedCount = newCount;
  if (state.pubkey) saveJson(uploadedKey(state.pubkey), newCount);
  return uploaded;
}

/** 全链自检（§4.1/§4.2/§4.3）。 */
export async function verifyLocalChain(): Promise<boolean> {
  const wasm = await loadWasm();
  return wasm.verify_chain(state.crumbs);
}

/** 清空本地链。 */
export function resetChain(): void {
  if (!state.pubkey) return;
  state.crumbs = [];
  state.coords = [];
  state.uploadedCount = 0;
  saveJson(chainKey(state.pubkey), []);
  saveJson(coordsKey(state.pubkey), []);
  saveJson(uploadedKey(state.pubkey), 0);
}
