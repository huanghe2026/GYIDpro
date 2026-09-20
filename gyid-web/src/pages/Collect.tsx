// Collect 页：GPS 面包屑采集。
//
// 流程（draft-04 §4）：
// 1. navigator.geolocation.watchPosition 持续拿 (lat,lng)；桌面无 GPS 时可切手动坐标
// 2. wasm h3_to_cell_hex 预览 H3 量化结果（分辨率 7..=10）
// 3. 计时：普通模式 900s（§4.1 15 分钟硬下限）/ 探索模式 300s（§4.2 允许 5..15 分钟）
// 4. 到点（或手动按钮）→ wasm collect_breadcrumb 签名，index/prev_hash 自动续链
// 5. chain store 立即 POST /v1/evidence（CBOR 帧流，300/批）；失败本地保留、断点续传

import { createResource, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import UnlockGate from "../components/UnlockGate";
import MapView, { type LatLng } from "../components/MapView";
import { identityStore } from "../stores/identity";
import {
  chainStore,
  collectNext,
  loadChain,
  resetChain,
  uploadPending,
  verifyLocalChain,
} from "../stores/chain";
import { loadWasm } from "../lib/wasm";
import { uploadEvidence } from "../lib/verifier";

/** §4.1 默认采集间隔（秒）—— 15 分钟硬下限 */
const NORMAL_INTERVAL = 900;
/** §4.2 探索会话最短间隔 —— 5 分钟 */
const EXPLORE_INTERVAL = 300;

const pad2 = (n: number) => String(n).padStart(2, "0");
const fmtCountdown = (sec: number) =>
  `${Math.floor(sec / 60)}:${pad2(sec % 60)}`;

const short = (h: string) =>
  h.length > 20 ? `${h.slice(0, 10)}…${h.slice(-8)}` : h;

export default function Collect() {
  return (
    <UnlockGate>
      <CollectInner />
    </UnlockGate>
  );
}

function CollectInner() {
  // ----- GPS / 手动坐标 -----
  const [gps, setGps] = createSignal<LatLng | null>(null);
  const [gpsErr, setGpsErr] = createSignal<string | null>(null);
  const [manual, setManual] = createSignal(false);
  const [manualLat, setManualLat] = createSignal("39.9042");
  const [manualLng, setManualLng] = createSignal("116.4074");

  // ----- 采集参数 -----
  const [resolution, setResolution] = createSignal(10);
  const [exploration, setExploration] = createSignal(false);
  const [autoCollect, setAutoCollect] = createSignal(true);

  // ----- 运行状态 -----
  const [now, setNow] = createSignal(Math.floor(Date.now() / 1000));
  const [busy, setBusy] = createSignal(false);
  const [logs, setLogs] = createSignal<string[]>([]);
  const [chainOk, setChainOk] = createSignal<boolean | null>(null);

  let watchId: number | null = null;
  let tickTimer: number | null = null;

  const log = (msg: string) => {
    const t = new Date().toLocaleTimeString();
    setLogs((l) => [`[${t}] ${msg}`, ...l].slice(0, 50));
  };

  // ----- 派生值 -----
  const currentCoords = (): LatLng | null => {
    if (manual()) {
      const lat = Number(manualLat());
      const lng = Number(manualLng());
      if (!Number.isFinite(lat) || !Number.isFinite(lng)) return null;
      return { lat, lng };
    }
    return gps();
  };

  const intervalSec = () => (exploration() ? EXPLORE_INTERVAL : NORMAL_INTERVAL);

  const lastTs = () =>
    chainStore.crumbs[chainStore.crumbs.length - 1]?.timestamp ?? null;

  const remainingSec = () => {
    const last = lastTs();
    if (last === null) return 0; // 空链随时可采集创世块
    return Math.max(0, last + intervalSec() - now());
  };

  const pendingCount = () =>
    chainStore.crumbs.length - chainStore.uploadedCount;

  // 当前坐标 → H3 cell hex 预览（坐标/分辨率变化时重算）
  const [cellHex] = createResource(
    () => {
      const c = currentCoords();
      return c
        ? `${c.lat.toFixed(7)},${c.lng.toFixed(7)},${resolution()}|${manual()}`
        : null;
    },
    async () => {
      const c = currentCoords();
      if (!c) return null;
      const wasm = await loadWasm();
      return wasm.h3_to_cell_hex(c.lat, c.lng, resolution());
    },
  );

  // ----- 采集 + 上传 -----
  const doCollect = async () => {
    if (busy()) return;
    const s = identityStore.session;
    if (!s) return;
    const c = currentCoords();
    if (!c) {
      log("✗ 无可用坐标");
      return;
    }
    setBusy(true);
    try {
      const crumb = await collectNext({
        seedHex: s.seedHex,
        lat: c.lat,
        lng: c.lng,
        resolution: resolution(),
        exploration: exploration(),
      });
      log(`✓ #${crumb.index} 已签名 (${c.lat.toFixed(5)}, ${c.lng.toFixed(5)})`);
      setChainOk(null);
      try {
        const n = await uploadPending(uploadEvidence);
        if (n > 0) log(`✓ 上传 ${n} 条到 Verifier`);
      } catch (e) {
        log(`✗ 上传失败：${String(e)}（本地保留，下次自动续传）`);
      }
    } catch (e) {
      log(`✗ 采集失败：${String(e)}`);
    } finally {
      setBusy(false);
    }
  };

  const doRetryUpload = async () => {
    if (busy()) return;
    setBusy(true);
    try {
      const n = await uploadPending(uploadEvidence);
      log(n > 0 ? `✓ 补传 ${n} 条` : "所有面包屑均已同步");
    } catch (e) {
      log(`✗ 补传失败：${String(e)}`);
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

  const doReset = () => {
    if (confirm("清空本账户的本地面包屑链？（Verifier 上已上传的记录不受影响）")) {
      resetChain();
      setChainOk(null);
      log("本地链已清空");
    }
  };

  // ----- 生命周期：GPS 订阅 + 每秒计时/到点自动采集 -----
  onMount(() => {
    const s = identityStore.session;
    if (s) loadChain(s.pubkeyHex);

    if (!("geolocation" in navigator)) {
      setGpsErr("浏览器不支持 Geolocation API，已切换手动坐标");
      setManual(true);
    } else {
      watchId = navigator.geolocation.watchPosition(
        (pos) => {
          setGps({
            lat: pos.coords.latitude,
            lng: pos.coords.longitude,
            accuracy: pos.coords.accuracy,
          });
          setGpsErr(null);
        },
        (err) => setGpsErr(`GPS 不可用：${err.message}（可改用手动坐标）`),
        { enableHighAccuracy: true, timeout: 15000, maximumAge: 5000 },
      );
    }

    tickTimer = window.setInterval(() => {
      setNow(Math.floor(Date.now() / 1000));
      if (autoCollect() && remainingSec() <= 0 && !busy() && currentCoords()) {
        void doCollect();
      }
    }, 1000);
  });

  onCleanup(() => {
    if (watchId !== null) navigator.geolocation.clearWatch(watchId);
    if (tickTimer !== null) clearInterval(tickTimer);
  });

  const session = () => identityStore.session;

  return (
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold">Collect</h1>
          <p class="text-xs text-gray-500 mt-0.5 font-mono">
            attester: {session() ? short(session()!.pubkeyHex) : "—"}
          </p>
        </div>
        <div class="flex items-center gap-4 text-sm">
          <label class="flex items-center gap-1.5 cursor-pointer">
            <input
              type="checkbox"
              checked={exploration()}
              onChange={(e) => setExploration(e.currentTarget.checked)}
            />
            <span>探索模式（5–15 分钟）</span>
          </label>
          <label class="flex items-center gap-1.5 cursor-pointer">
            <input
              type="checkbox"
              checked={autoCollect()}
              onChange={(e) => setAutoCollect(e.currentTarget.checked)}
            />
            <span>自动采集</span>
          </label>
        </div>
      </div>

      <div class="grid gap-6 lg:grid-cols-3">
        {/* 左：控制面板 */}
        <div class="space-y-4">
          <div class="bg-white rounded-lg shadow p-4 space-y-3">
            <h2 class="font-semibold text-sm text-gray-700">定位</h2>
            <Show
              when={!manual()}
              fallback={
                <div>
                  <p class="text-xs text-gray-500 mb-2">手动坐标模式（桌面调试）</p>
                  <div class="flex gap-2">
                    <input
                      class="w-full border rounded px-2 py-1.5 text-sm"
                      value={manualLat()}
                      onInput={(e) => setManualLat(e.currentTarget.value)}
                      placeholder="lat"
                    />
                    <input
                      class="w-full border rounded px-2 py-1.5 text-sm"
                      value={manualLng()}
                      onInput={(e) => setManualLng(e.currentTarget.value)}
                      placeholder="lng"
                    />
                  </div>
                </div>
              }
            >
              <Show
                when={gps()}
                fallback={
                  <p class="text-sm text-gray-500">
                    等待 GPS 定位…（需浏览器定位权限）
                  </p>
                }
              >
                <p class="text-sm">
                  <span class="text-green-700">● GPS</span>{" "}
                  {gps()!.lat.toFixed(6)}, {gps()!.lng.toFixed(6)}
                </p>
                <p class="text-xs text-gray-500">
                  精度 ±{Math.round(gps()!.accuracy ?? NaN)}m
                </p>
              </Show>
            </Show>
            <Show when={gpsErr()}>
              <p class="text-xs text-amber-700">{gpsErr()}</p>
            </Show>
            <button
              class="text-xs text-blue-600 hover:underline"
              onClick={() => setManual((v) => !v)}
            >
              {manual() ? "切回 GPS" : "改用手动坐标"}
            </button>
          </div>

          <div class="bg-white rounded-lg shadow p-4 space-y-3">
            <h2 class="font-semibold text-sm text-gray-700">采集参数</h2>
            <label class="block">
              <span class="text-xs text-gray-500">H3 分辨率（7..=10）</span>
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
            <div>
              <span class="text-xs text-gray-500">当前 H3 cell（hex）</span>
              <code class="block mt-1 bg-gray-100 rounded px-2 py-1.5 text-xs break-all">
                <Show when={cellHex()} fallback={"—"}>
                  {cellHex()}
                </Show>
              </code>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-xs text-gray-500">
                间隔 {intervalSec()}s · 距下次可采集
              </span>
              <span
                class="font-mono text-lg font-semibold"
                classList={{
                  "text-green-600": remainingSec() === 0,
                  "text-gray-800": remainingSec() > 0,
                }}
              >
                {remainingSec() === 0 ? "ready" : fmtCountdown(remainingSec())}
              </span>
            </div>
            <button
              class="w-full bg-blue-600 text-white text-sm rounded py-2 font-medium hover:bg-blue-700 disabled:opacity-50"
              disabled={busy() || !currentCoords()}
              onClick={doCollect}
            >
              {busy() ? "处理中…" : "立即采集并签名（手动）"}
            </button>
          </div>

          <div class="bg-white rounded-lg shadow p-4 space-y-3">
            <h2 class="font-semibold text-sm text-gray-700">本地链</h2>
            <div class="grid grid-cols-3 gap-2 text-center">
              <MiniStat label="本地" value={chainStore.crumbs.length} />
              <MiniStat label="已上传" value={chainStore.uploadedCount} />
              <MiniStat label="待同步" value={pendingCount()} />
            </div>
            <Show when={chainOk() !== null}>
              <p
                class="text-sm"
                classList={{
                  "text-green-700": chainOk() === true,
                  "text-red-700": chainOk() === false,
                }}
              >
                全链自检：{chainOk() ? "✓ 通过" : "✗ 违规（见 console）"}
              </p>
            </Show>
            <div class="flex gap-2">
              <button
                class="flex-1 text-xs border rounded py-1.5 hover:bg-gray-50 disabled:opacity-50"
                disabled={busy() || pendingCount() === 0}
                onClick={doRetryUpload}
              >
                续传未同步
              </button>
              <button
                class="flex-1 text-xs border rounded py-1.5 hover:bg-gray-50 disabled:opacity-50"
                disabled={busy() || chainStore.crumbs.length === 0}
                onClick={doVerify}
              >
                全链自检
              </button>
              <button
                class="flex-1 text-xs border rounded py-1.5 text-red-600 hover:bg-red-50"
                onClick={doReset}
              >
                清空
              </button>
            </div>
          </div>
        </div>

        {/* 右：地图 + 日志 */}
        <div class="lg:col-span-2 space-y-4">
          <div class="bg-white rounded-lg shadow p-4">
            <h2 class="font-semibold text-sm text-gray-700 mb-3">
              轨迹（蓝点 = 面包屑，红点 = 当前定位）
            </h2>
            <MapView coords={chainStore.coords} current={currentCoords()} />
          </div>
          <div class="bg-white rounded-lg shadow p-4">
            <h2 class="font-semibold text-sm text-gray-700 mb-2">采集日志</h2>
            <div class="font-mono text-xs space-y-1 max-h-48 overflow-y-auto">
              <Show
                when={logs().length > 0}
                fallback={<p class="text-gray-400">暂无日志</p>}
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

function MiniStat(props: { label: string; value: number }) {
  return (
    <div class="border rounded py-2">
      <div class="text-lg font-semibold">{props.value}</div>
      <div class="text-xs text-gray-500">{props.label}</div>
    </div>
  );
}
