// Import 页：手动导入 GPX / 照片轨迹（GYIP-0003 路线图 W6 验收项）。
//
// 适用场景：桌面端没有 GPS，或用户已有历史轨迹（运动手表/手机导出的 GPX、
// 带 GPS 的照片）。不依赖实时定位，直接把文件转成**已签名**的面包屑链。
//
// 流程：
// 1. 选 GPX（XML）或一批带 GPS 的 JPEG → lib/importers 解析出候选点 (lat,lng,ts)；
// 2. gyid-wasm 量化 H3 + §4 链规则过滤 → previewImport 展示「接受 / 丢弃」明细；
// 3. importCrumbs 逐条 Ed25519 签名追加到本地链（index / prev_hash 自动续接）；
// 4. uploadPending 批量上传 Verifier，随后可全链自检 / 走 Verify 页换 PoH。
//
// 隐私：原始经纬度只留在本地（内存 + localStorage）供地图展示；上传 Verifier 的
// 只有 H3 cell 与统计字段（draft-04 §4 强制"原始 GPS 不出端"）。

import { createMemo, createResource, createSignal, For, onMount, Show } from "solid-js";
import UnlockGate from "../components/UnlockGate";
import MapView from "../components/MapView";
import { identityStore } from "../stores/identity";
import {
  chainStore,
  importCrumbs,
  loadChain,
  previewImport,
  uploadPending,
  verifyLocalChain,
  type ImportResult,
  type ImportSkipReason,
} from "../stores/chain";
import {
  parseGpx,
  parsePhotoFiles,
  type GpxParseResult,
  type PhotoParseResult,
  type TrackPoint,
} from "../lib/importers";
import { uploadEvidence } from "../lib/verifier";
import { fmtDateTime, t } from "../i18n";

/** 丢弃原因 → 当前语言的可读说明。 */
function reasonLabel(reason: ImportSkipReason): string {
  switch (reason) {
    case "same-cell":
      return t("imp.reasonSameCell");
    case "cell-cap":
      return t("imp.reasonCellCap");
    case "too-soon":
      return t("imp.reasonTooSoon");
    case "time-backwards":
      return t("imp.reasonTimeBackwards");
  }
}

export default function Import() {
  return (
    <UnlockGate>
      <ImportInner />
    </UnlockGate>
  );
}

function ImportInner() {
  const session = () => identityStore.session;

  // 解析结果
  const [gpx, setGpx] = createSignal<{ name: string; result: GpxParseResult } | null>(null);
  const [photos, setPhotos] = createSignal<{ names: string[]; result: PhotoParseResult } | null>(null);

  // 参数
  const [resolution, setResolution] = createSignal(10);
  const [exploration, setExploration] = createSignal(false);
  const [syntheticInterval, setSyntheticInterval] = createSignal(900);

  // 运行状态
  const [busy, setBusy] = createSignal(false);
  const [progress, setProgress] = createSignal<string | null>(null);
  const [err, setErr] = createSignal<string | null>(null);
  const [logs, setLogs] = createSignal<string[]>([]);
  const [importResult, setImportResult] = createSignal<ImportResult | null>(null);
  const [chainOk, setChainOk] = createSignal<boolean | null>(null);

  const log = (msg: string) => {
    const time = new Date().toLocaleTimeString();
    setLogs((l) => [`[${time}] ${msg}`, ...l].slice(0, 60));
  };

  /** 合并 GPX + 照片候选点（按时间升序）。 */
  const candidates = createMemo<TrackPoint[]>(() => {
    const pts: TrackPoint[] = [];
    for (const p of gpx()?.result.points ?? []) pts.push(p);
    for (const p of photos()?.result.points ?? []) pts.push(p);
    return pts.sort((a, b) => a.ts - b.ts);
  });

  // 参数 / 候选点 / 链长变化即重新预演（WASM 量化 + 链规则过滤）
  const [preview] = createResource(
    () => {
      const pts = candidates();
      if (pts.length === 0) return null;
      return `${pts.length}:${resolution()}:${exploration()}:${chainStore.crumbs.length}:${session()?.pubkeyHex ?? ""}`;
    },
    async () => {
      const pts = candidates();
      if (pts.length === 0) return null;
      return previewImport({
        points: pts.map((p) => ({ lat: p.lat, lng: p.lng, ts: p.ts })),
        resolution: resolution(),
        exploration: exploration(),
      });
    },
  );

  const skipCounts = createMemo(() => {
    const m = new Map<ImportSkipReason, number>();
    for (const s of preview()?.skipped ?? []) {
      m.set(s.reason, (m.get(s.reason) ?? 0) + 1);
    }
    return m;
  });

  const pending = () => chainStore.crumbs.length - chainStore.uploadedCount;
  const tailTs = () => {
    const last = chainStore.crumbs[chainStore.crumbs.length - 1];
    return last ? last.timestamp : null;
  };
  const timeRange = () => {
    const pts = candidates();
    if (pts.length === 0) return null;
    return [pts[0].ts, pts[pts.length - 1].ts] as const;
  };

  onMount(() => {
    const s = session();
    if (s) loadChain(s.pubkeyHex);
  });

  // ── 文件选择 ──
  const onGpxFile = async (e: Event) => {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    setBusy(true);
    setErr(null);
    setGpx(null);
    setImportResult(null);
    setChainOk(null);
    try {
      const text = await file.text();
      const result = parseGpx(text, {
        syntheticIntervalSec: syntheticInterval(),
      });
      setGpx({ name: file.name, result });
      log(
        t("imp.logGpx", {
          name: file.name,
          n: result.points.length,
          invalid: result.invalid,
          syn: result.synthesized,
        }),
      );
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : String(e2));
    } finally {
      setBusy(false);
    }
  };

  const onPhotoFiles = async (e: Event) => {
    const input = e.currentTarget as HTMLInputElement;
    const files = Array.from(input.files ?? []);
    input.value = "";
    if (files.length === 0) return;
    setBusy(true);
    setErr(null);
    setPhotos(null);
    setImportResult(null);
    setChainOk(null);
    try {
      const result = await parsePhotoFiles(files);
      setPhotos({ names: files.map((f) => f.name), result });
      log(
        t("imp.logPhotosOk", { n: result.points.length, total: files.length }) +
          (result.failed.length > 0
            ? t("imp.logPhotosFailSuffix", { f: result.failed.length })
            : ""),
      );
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : String(e2));
    } finally {
      setBusy(false);
    }
  };

  // ── 导入 / 上传 / 自检 ──
  const doImport = async () => {
    const s = session();
    const pts = candidates();
    if (!s || pts.length === 0 || busy()) return;
    setBusy(true);
    setErr(null);
    setProgress(null);
    try {
      const res = await importCrumbs({
        seedHex: s.seedHex,
        points: pts.map((p) => ({ lat: p.lat, lng: p.lng, ts: p.ts })),
        resolution: resolution(),
        exploration: exploration(),
        onProgress: (done, total) => setProgress(`${done}/${total}`),
      });
      setImportResult(res);
      setChainOk(null);
      log(t("imp.logImportDone", { a: res.added, s: res.skipped.length }));
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : String(e2));
    } finally {
      setBusy(false);
      setProgress(null);
    }
  };

  const doUpload = async () => {
    if (busy()) return;
    setBusy(true);
    try {
      const n = await uploadPending(uploadEvidence);
      log(n > 0 ? t("imp.logUploaded", { n }) : t("imp.logAllSynced"));
    } catch (e2) {
      log(t("imp.logUploadFail", { e: String(e2) }));
    } finally {
      setBusy(false);
    }
  };

  const doVerify = async () => {
    if (chainStore.crumbs.length === 0) {
      setChainOk(null);
      return;
    }
    setBusy(true);
    try {
      setChainOk(await verifyLocalChain());
    } finally {
      setBusy(false);
    }
  };

  return (
    <div class="space-y-6">
      <div>
        <h1 class="text-2xl font-bold">Import</h1>
        <p class="text-xs text-gray-500 mt-0.5">
          {t("imp.subtitle")}
        </p>
      </div>

      <Show when={err()}>
        <div class="bg-red-50 border border-red-200 text-red-700 text-sm rounded px-3 py-2">
          {err()}
        </div>
      </Show>

      <div class="grid gap-6 lg:grid-cols-3">
        {/* 左：来源与参数 */}
        <div class="space-y-4">
          <div class="bg-white rounded-lg shadow p-4 space-y-3">
            <h2 class="font-semibold text-sm text-gray-700">{t("imp.fileTitle")}</h2>
            <label class="block">
              <span class="text-xs text-gray-500">{t("imp.gpxLabel")}</span>
              <input
                type="file"
                accept=".gpx,application/gpx+xml,application/xml,text/xml"
                class="mt-1 block w-full text-xs"
                onChange={onGpxFile}
              />
            </label>
            <Show when={gpx()}>
              <p class="text-xs text-green-700">
                {t("imp.gpxReady", { name: gpx()!.name, n: gpx()!.result.points.length })}
              </p>
            </Show>
            <label class="block border-t pt-3">
              <span class="text-xs text-gray-500">
                {t("imp.photoLabel")}
              </span>
              <input
                type="file"
                accept="image/jpeg,.jpg,.jpeg"
                multiple
                class="mt-1 block w-full text-xs"
                onChange={onPhotoFiles}
              />
            </label>
            <Show when={photos()}>
              <p class="text-xs text-green-700">
                {t("imp.photosReady", { n: photos()!.result.points.length })}
              </p>
            </Show>
          </div>

          <div class="bg-white rounded-lg shadow p-4 space-y-3">
            <h2 class="font-semibold text-sm text-gray-700">{t("imp.paramsTitle")}</h2>
            <label class="block">
              <span class="text-xs text-gray-500">{t("imp.h3Resolution")}</span>
              <select
                class="w-full border rounded px-2 py-1.5 text-sm mt-1"
                value={resolution()}
                onChange={(e) => setResolution(Number(e.currentTarget.value))}
              >
                <For each={[7, 8, 9, 10]}>
                  {(r) => <option value={r}>res {r}</option>}
                </For>
              </select>
            </label>
            <label class="block">
              <span class="text-xs text-gray-500">
                {t("imp.synthInterval")}
              </span>
              <input
                type="number"
                min="300"
                step="60"
                class="w-full border rounded px-2 py-1.5 text-sm mt-1"
                value={syntheticInterval()}
                onInput={(e) =>
                  setSyntheticInterval(Number(e.currentTarget.value) || 900)
                }
              />
            </label>
            <label class="flex items-center gap-1.5 text-sm cursor-pointer">
              <input
                type="checkbox"
                checked={exploration()}
                onChange={(e) => setExploration(e.currentTarget.checked)}
              />
              <span>{t("imp.exploration")}</span>
            </label>
            <p class="text-[11px] text-gray-400 leading-relaxed">
              {t("imp.tzNote")}
            </p>
          </div>
        </div>

        {/* 中：预演与操作 */}
        <div class="space-y-4">
          <div class="bg-white rounded-lg shadow p-4 space-y-3">
            <h2 class="font-semibold text-sm text-gray-700">{t("imp.previewTitle")}</h2>
            <div class="grid grid-cols-3 gap-2 text-center">
              <MiniStat label={t("imp.miniCandidates")} value={candidates().length} />
              <MiniStat
                label={t("imp.miniAccepted")}
                value={preview()?.accepted ?? 0}
                tone="green"
              />
              <MiniStat
                label={t("imp.miniRejected")}
                value={preview()?.skipped.length ?? 0}
                tone="red"
              />
            </div>
            <Show when={preview.loading}>
              <p class="text-xs text-gray-500">{t("imp.quantizing")}</p>
            </Show>
            <Show when={timeRange()}>
              {(r) => (
                <p class="text-[11px] text-gray-500">
                  {t("imp.timeRange", {
                    a: fmtDateTime(r()[0] * 1000),
                    b: fmtDateTime(r()[1] * 1000),
                  })}
                </p>
              )}
            </Show>
            <Show when={(preview()?.skipped.length ?? 0) > 0}>
              <div class="border rounded p-2 space-y-1">
                <p class="text-xs font-medium text-gray-700">{t("imp.reasonsTitle")}</p>
                <For each={Array.from(skipCounts().entries())}>
                  {([reason, count]) => (
                    <p class="text-[11px] text-gray-600">
                      {count} × {reasonLabel(reason)}
                    </p>
                  )}
                </For>
              </div>
            </Show>
            <button
              class="w-full bg-blue-600 text-white text-sm rounded py-2 font-medium hover:bg-blue-700 disabled:opacity-50"
              disabled={busy() || !session() || (preview()?.accepted ?? 0) === 0}
              onClick={doImport}
            >
              {progress()
                ? t("imp.signing", { progress: progress()! })
                : t("imp.importBtn", { n: preview()?.accepted ?? 0 })}
            </button>
            <Show when={importResult()}>
              <p class="text-xs text-green-700">
                {t("imp.imported", { n: importResult()!.added })}
              </p>
            </Show>
          </div>

          <div class="bg-white rounded-lg shadow p-4 space-y-3">
            <h2 class="font-semibold text-sm text-gray-700">{t("imp.syncTitle")}</h2>
            <div class="grid grid-cols-3 gap-2 text-center">
              <MiniStat label={t("imp.miniLocal")} value={chainStore.crumbs.length} />
              <MiniStat label={t("imp.miniUploaded")} value={chainStore.uploadedCount} />
              <MiniStat label={t("imp.miniPending")} value={pending()} />
            </div>
            <Show when={tailTs() !== null}>
              <p class="text-[11px] text-gray-500">
                {t("imp.tailTs", { t: fmtDateTime(tailTs()! * 1000) })}
              </p>
            </Show>
            <Show when={chainOk() !== null}>
              <p
                class="text-sm"
                classList={{
                  "text-green-700": chainOk() === true,
                  "text-red-700": chainOk() === false,
                }}
              >
                {t("imp.checkPre")}{chainOk() ? t("imp.checkOk") : t("imp.checkBad")}
              </p>
            </Show>
            <div class="flex gap-2">
              <button
                class="flex-1 text-xs border rounded py-1.5 hover:bg-gray-50 disabled:opacity-50"
                disabled={busy() || pending() === 0}
                onClick={doUpload}
              >
                {t("imp.uploadBtn")}
              </button>
              <button
                class="flex-1 text-xs border rounded py-1.5 hover:bg-gray-50 disabled:opacity-50"
                disabled={busy() || chainStore.crumbs.length === 0}
                onClick={doVerify}
              >
                {t("imp.verifyChain")}
              </button>
            </div>
          </div>

          <Show when={photos() && photos()!.result.failed.length > 0}>
            <div class="bg-white rounded-lg shadow p-4 space-y-1">
              <h2 class="font-semibold text-sm text-gray-700">{t("imp.failedTitle")}</h2>
              <For each={photos()!.result.failed.slice(0, 10)}>
                {(f) => (
                  <p class="text-[11px] text-gray-600">
                    {f.name} — {f.reason}
                  </p>
                )}
              </For>
            </div>
          </Show>
        </div>

        {/* 右：地图 + 日志 */}
        <div class="space-y-4">
          <div class="bg-white rounded-lg shadow p-4">
            <h2 class="font-semibold text-sm text-gray-700 mb-3">
              {t("imp.mapTitle")}
            </h2>
            <MapView coords={chainStore.coords} current={null} />
          </div>
          <div class="bg-white rounded-lg shadow p-4">
            <h2 class="font-semibold text-sm text-gray-700 mb-2">{t("imp.logTitle")}</h2>
            <div class="font-mono text-xs space-y-1 max-h-48 overflow-y-auto">
              <Show
                when={logs().length > 0}
                fallback={<p class="text-gray-400">{t("imp.noLogs")}</p>}
              >
                <For each={logs()}>{(line) => <p>{line}</p>}</For>
              </Show>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function MiniStat(props: {
  label: string;
  value: number;
  tone?: "green" | "red";
}) {
  return (
    <div class="border rounded py-2">
      <div
        class="text-lg font-semibold"
        classList={{
          "text-green-600": props.tone === "green",
          "text-red-600": props.tone === "red",
        }}
      >
        {props.value}
      </div>
      <div class="text-xs text-gray-500">{props.label}</div>
    </div>
  );
}
