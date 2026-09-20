// Verify 页：Active Verification 全流程（draft-04 §12）。
//
// 本页同时扮演两个角色：
// - Attester：解锁身份 → WS 收挑战 → wasm 签名回送；
// - RP：生成 rp_nonce → POST /v1/verify → 轮询 PoH → wasm verify_poh 本地校验。
//
// 流程：连接 WS（等 ready 文本帧）→ 请求挑战 → 收二进制 LivenessChallenge →
// sign_liveness_response 回送 → 轮询 POST /v1/poh（202=Pending）→ verify_poh 展示。
// 注意：挑战期间链头不得变化（Collect 页采集会使挑战失效，服务端拒绝签发）。

import { createSignal, For, onCleanup, Show } from "solid-js";
import UnlockGate from "../components/UnlockGate";
import { identityStore } from "../stores/identity";
import {
  bytesToHex,
  fetchPoh,
  fetchVerifierPubkeyHex,
  hexToBytes,
  openChallengeWs,
  requestChallenge,
} from "../lib/verifier";
import { loadWasm, type LivenessChallengeJs, type PohInfoJs } from "../lib/wasm";

/** 策略门槛（与 trip-server Config::default 对齐：min_confidence=0.1 / min_trust=20.0） */
const MIN_CONFIDENCE = 0.1;
const MIN_TRUST = 20.0;

type Phase =
  | "idle"
  | "connecting"
  | "requesting"
  | "waiting"
  | "responding"
  | "polling"
  | "done"
  | "error";

const PHASE_ORDER: Phase[] = [
  "connecting",
  "requesting",
  "waiting",
  "responding",
  "polling",
];

const STEPS = [
  "连接 Verifier WebSocket",
  "请求挑战（POST /v1/verify）",
  "接收 LivenessChallenge",
  "签名回送 LivenessResponse",
  "轮询取回 PoH 证书",
  "本地 verify_poh 校验",
];

const short = (h: string) =>
  h.length > 20 ? `${h.slice(0, 10)}…${h.slice(-8)}` : h;

const fmtTs = (sec: number) => new Date(sec * 1000).toLocaleTimeString();

/** 生成 16 字节随机 rp_nonce hex（RP 侧防重放，draft-04 §10 §1） */
function genNonce(): string {
  return bytesToHex(crypto.getRandomValues(new Uint8Array(16)));
}

// ---------------------------------------------------------------------------
// 竞态安全的 WS 收包通道：构造时即挂监听，消息进队列 / 直接唤醒 waiter，
// 避免「open 事件与 ready 文本帧之间丢失消息」的经典竞态。
// ---------------------------------------------------------------------------
interface WsChan {
  ws: WebSocket;
  queue: MessageEvent[];
  waiters: ((ev: MessageEvent | null) => void)[];
  closed: boolean;
}

function openChan(attesterHex: string): WsChan {
  const ws = openChallengeWs(attesterHex);
  const chan: WsChan = { ws, queue: [], waiters: [], closed: false };
  ws.addEventListener("message", (ev) => {
    const w = chan.waiters.shift();
    if (w) w(ev);
    else chan.queue.push(ev);
  });
  ws.addEventListener("close", () => {
    chan.closed = true;
    while (chan.waiters.length) chan.waiters.shift()!(null);
  });
  return chan;
}

/** 取一条消息；队列空则等待，超时/断开返回 null */
function takeMessage(chan: WsChan, timeoutMs: number): Promise<MessageEvent | null> {
  const buffered = chan.queue.shift();
  if (buffered) return Promise.resolve(buffered);
  return new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      const i = chan.waiters.indexOf(wake);
      if (i >= 0) chan.waiters.splice(i, 1);
      resolve(null);
    }, timeoutMs);
    const wake = (ev: MessageEvent | null) => {
      window.clearTimeout(timer);
      resolve(ev);
    };
    chan.waiters.push(wake);
  });
}

export default function Verify() {
  return (
    <UnlockGate>
      <VerifyInner />
    </UnlockGate>
  );
}

function VerifyInner() {
  const [phase, setPhase] = createSignal<Phase>("idle");
  const [error, setError] = createSignal<string | null>(null);
  const [logs, setLogs] = createSignal<string[]>([]);
  const [nonce, setNonce] = createSignal(genNonce());
  const [challenge, setChallenge] = createSignal<LivenessChallengeJs | null>(null);
  const [meta, setMeta] = createSignal<{
    id: string;
    expiresAt: number;
    delivered: boolean;
  } | null>(null);
  const [poh, setPoh] = createSignal<PohInfoJs | null>(null);
  const [pohHex, setPohHex] = createSignal<string | null>(null);

  let chan: WsChan | null = null;
  const timers: number[] = [];

  const log = (msg: string) => {
    const t = new Date().toLocaleTimeString();
    setLogs((l) => [`[${t}] ${msg}`, ...l].slice(0, 50));
  };

  const sleep = (ms: number) =>
    new Promise<void>((r) => timers.push(window.setTimeout(r, ms)));

  onCleanup(() => {
    chan?.ws.close();
    timers.forEach(clearTimeout);
  });

  const running = () => PHASE_ORDER.includes(phase());

  const stepState = (idx: number): "done" | "active" | "todo" => {
    if (phase() === "done") return "done";
    const cur = PHASE_ORDER.indexOf(phase());
    if (cur < 0) return "todo";
    if (idx < cur) return "done";
    if (idx === cur) return "active";
    return "todo";
  };

  const run = async () => {
    const s = identityStore.session;
    if (!s || running()) return;
    setError(null);
    setChallenge(null);
    setMeta(null);
    setPoh(null);
    setPohHex(null);
    try {
      // ---- 1) 连接 WS，等 ready 文本帧（注册完成，可发起挑战）----
      setPhase("connecting");
      log("连接 Verifier WS…");
      chan?.ws.close();
      chan = openChan(s.pubkeyHex);
      const ready = await takeMessage(chan, 10_000);
      if (!ready) throw new Error("等待 WS ready 超时（检查 Verifier 是否运行）");
      log(`WS ready：${typeof ready.data === "string" ? ready.data : "(binary?)"}`);

      // ---- 2) RP 请求挑战 ----
      setPhase("requesting");
      const rpNonce = nonce();
      log(`POST /v1/verify（rp_nonce=${rpNonce.slice(0, 8)}…）`);
      const info = await requestChallenge(s.pubkeyHex, rpNonce);
      setMeta({ id: info.challenge_id, expiresAt: info.expires_at, delivered: info.delivered });
      log(
        `挑战 ${info.challenge_id.slice(0, 8)}… 已创建，过期 ${fmtTs(info.expires_at)}`,
      );
      if (!info.delivered) {
        throw new Error("挑战未被推送到 WS（delivered=false），请重试");
      }

      // ---- 3) 收二进制挑战帧（忽略期间的文本帧）----
      setPhase("waiting");
      const waitMs = Math.max((info.expires_at * 1000 - Date.now()) | 0, 10_000) + 5_000;
      let frame = await takeMessage(chan, waitMs);
      while (frame && typeof frame.data === "string") {
        log(`WS 文本：${frame.data}`);
        frame = await takeMessage(chan, waitMs);
      }
      if (!frame) throw new Error("等待挑战超时");
      const challengeHex = bytesToHex(new Uint8Array(frame.data as ArrayBuffer));
      const wasm = await loadWasm();
      const ch = wasm.parse_liveness_challenge(challengeHex);
      setChallenge(ch);
      log(
        `收到挑战：expected_index=${ch.expected_index}，deadline=${fmtTs(ch.deadline)}`,
      );

      // ---- 4) Attester 签名回送 ----
      setPhase("responding");
      const respHex = wasm.sign_liveness_response(s.seedHex, challengeHex);
      chan.ws.send(hexToBytes(respHex));
      log("已回送 LivenessResponse（CBOR 二进制帧）");

      // ---- 5) 轮询 PoH（202=Pending，1s 间隔）----
      setPhase("polling");
      let pohBytes: Uint8Array | null = null;
      const pollDeadline = info.expires_at * 1000 + 15_000;
      while (Date.now() < pollDeadline) {
        // 顺带消费 WS 文本回执（poh_issued / error），仅入日志
        const ack = await takeMessage(chan, 0);
        if (ack && typeof ack.data === "string") log(`WS 回执：${ack.data}`);
        const r = await fetchPoh(info.challenge_id);
        if (r instanceof Uint8Array) {
          pohBytes = r;
          break;
        }
        await sleep(1000);
      }
      if (!pohBytes) throw new Error("轮询 PoH 超时（挑战可能未被处理）");
      const hex = bytesToHex(pohBytes);
      setPohHex(hex);
      log(`PoH 证书已取回（${pohBytes.length} 字节 CBOR）`);

      // ---- 6) RP 本地校验：Verifier 签名 + 新鲜性（nonce 绑定）+ 策略 ----
      const vk = await fetchVerifierPubkeyHex();
      const p = wasm.verify_poh(
        hex,
        vk,
        rpNonce,
        BigInt(Math.floor(Date.now() / 1000)),
        MIN_CONFIDENCE,
        MIN_TRUST,
      );
      setPoh(p);
      setPhase("done");
      log(
        p.is_trusted
          ? `✓ 校验通过：trust=${p.trust.toFixed(3)}，confidence=${p.criticality_confidence.toFixed(3)}`
          : `✗ 未通过：fresh=${p.fresh}，policy_pass=${p.policy_pass}`,
      );
    } catch (e) {
      setPhase("error");
      setError(String(e));
      log(`✗ ${String(e)}`);
    } finally {
      chan?.ws.close();
      chan = null;
    }
  };

  const stepDot = (st: "done" | "active" | "todo") =>
    st === "done"
      ? "bg-green-500"
      : st === "active"
        ? "bg-blue-500 animate-pulse"
        : "bg-gray-300";

  const p = () => poh();

  return (
    <div class="space-y-6">
      <div class="flex items-start justify-between gap-4">
        <div>
          <h1 class="text-2xl font-bold">Verify</h1>
          <p class="text-xs text-gray-500 mt-0.5 font-mono">
            attester: {identityStore.session ? short(identityStore.session.pubkeyHex) : "—"}
          </p>
        </div>
        <div class="flex items-center gap-2">
          <button
            class="text-xs border rounded px-2 py-1 text-gray-600 hover:bg-gray-50 disabled:opacity-40"
            disabled={running()}
            onClick={() => setNonce(genNonce())}
            title="RP 防重放 nonce（16 字节随机）"
          >
            换 nonce
          </button>
          <button
            class="bg-blue-600 text-white text-sm font-medium rounded px-4 py-1.5 hover:bg-blue-700 disabled:opacity-50"
            disabled={running()}
            onClick={() => void run()}
          >
            {running() ? "进行中…" : "发起验证"}
          </button>
        </div>
      </div>

      <p class="text-xs text-amber-700 bg-amber-50 border border-amber-200 rounded px-3 py-2">
        验证期间请勿在 Collect 页采集：挑战会绑定当前链头，链头变化后服务端拒绝签发。
        该身份需先在 Collect 页上传过面包屑，否则 POST /v1/verify 返回 404。
      </p>

      <div class="grid gap-6 lg:grid-cols-2">
        {/* 左：步骤 + 挑战详情 */}
        <div class="space-y-4">
          <div class="bg-white rounded-lg shadow p-4">
            <h2 class="font-semibold text-sm text-gray-700 mb-3">流程</h2>
            <ol class="space-y-2">
              <For each={STEPS}>
                {(label, i) => (
                  <li class="flex items-center gap-2 text-sm">
                    <span class={`w-2 h-2 rounded-full ${stepDot(stepState(i()))}`} />
                    <span class={stepState(i()) === "todo" ? "text-gray-400" : "text-gray-800"}>
                      {label}
                    </span>
                  </li>
                )}
              </For>
            </ol>
            <Show when={error()}>
              <p class="mt-3 text-sm text-red-600 break-all">{error()}</p>
            </Show>
          </div>

          <Show when={meta()}>
            <div class="bg-white rounded-lg shadow p-4 space-y-1 text-sm">
              <h2 class="font-semibold text-sm text-gray-700 mb-2">挑战</h2>
              <p class="font-mono text-xs text-gray-600">
                challenge_id: {meta()!.id}
              </p>
              <p class="text-gray-600">
                过期时间：{fmtTs(meta()!.expiresAt)} ｜ 推送：{meta()!.delivered ? "已送达 WS" : "未送达"}
              </p>
            </div>
          </Show>

          <Show when={challenge()}>
            <div class="bg-white rounded-lg shadow p-4 space-y-1 text-sm">
              <h2 class="font-semibold text-sm text-gray-700 mb-2">
                LivenessChallenge 字段
              </h2>
              <p class="font-mono text-xs text-gray-600">
                rp_nonce: {short(challenge()!.rp_nonce_hex)}（与本地生成一致）
              </p>
              <p class="font-mono text-xs text-gray-600 break-all">
                chain_head: {short(challenge()!.chain_head_hex)}
              </p>
              <p class="text-gray-600">
                expected_index: {challenge()!.expected_index} ｜ deadline: {fmtTs(challenge()!.deadline)}
              </p>
            </div>
          </Show>
        </div>

        {/* 右：PoH 结果 + 日志 */}
        <div class="space-y-4">
          <Show
            when={p()}
            fallback={
              <div class="bg-white rounded-lg shadow p-4 text-sm text-gray-400">
                PoH 证书将在此展示
              </div>
            }
          >
            <div class={`bg-white rounded-lg shadow p-4 border-l-4 ${p()!.is_trusted ? "border-green-500" : "border-red-500"}`}>
              <div class="flex items-center gap-2 mb-3">
                <span
                  class={`text-xs font-bold px-2 py-0.5 rounded ${
                    p()!.is_trusted
                      ? "bg-green-100 text-green-800"
                      : "bg-red-100 text-red-800"
                  }`}
                >
                  {p()!.is_trusted ? "FRESH + PASS" : "REJECTED"}
                </span>
                <span class="text-xs text-gray-500">
                  fresh: {String(p()!.fresh)} ｜ policy_pass: {String(p()!.policy_pass)}
                </span>
              </div>
              <dl class="grid grid-cols-2 gap-x-4 gap-y-1.5 text-sm">
                <dt class="text-gray-500">信任分 trust</dt>
                <dd class="font-mono text-right">{p()!.trust.toFixed(4)}</dd>
                <dt class="text-gray-500">置信度 confidence</dt>
                <dd class="font-mono text-right">{p()!.criticality_confidence.toFixed(4)}</dd>
                <dt class="text-gray-500">α / β / κ / π</dt>
                <dd class="font-mono text-right">
                  {p()!.alpha.toFixed(3)} / {p()!.beta.toFixed(3)} / {p()!.kappa.toFixed(1)} / {p()!.pi.toFixed(1)}
                </dd>
                <dt class="text-gray-500">面包屑 / 唯一 cell</dt>
                <dd class="font-mono text-right">
                  {p()!.breadcrumb_count} / {p()!.unique_cells}
                </dd>
                <dt class="text-gray-500">签发时间</dt>
                <dd class="text-right">{new Date(p()!.issued_at * 1000).toLocaleString()}</dd>
                <dt class="text-gray-500">有效期</dt>
                <dd class="text-right">{p()!.validity_secs}s（至 {new Date((p()!.issued_at + p()!.validity_secs) * 1000).toLocaleTimeString()}）</dd>
                <dt class="text-gray-500">nonce / chain_head</dt>
                <dd class="font-mono text-xs text-right break-all">
                  {short(p()!.nonce_hex)} / {short(p()!.chain_head_hex)}
                </dd>
              </dl>
              <Show when={pohHex()}>
                <p
                  class="mt-3 font-mono text-[10px] text-gray-400 break-all cursor-pointer hover:text-gray-600"
                  title="点击复制完整证书 CBOR hex"
                  onClick={() => void navigator.clipboard.writeText(pohHex()!)}
                >
                  CBOR: {pohHex()!.slice(0, 64)}…（点击复制）
                </p>
              </Show>
            </div>
          </Show>

          <div class="bg-white rounded-lg shadow p-4">
            <h2 class="font-semibold text-sm text-gray-700 mb-2">日志</h2>
            <div class="font-mono text-xs text-gray-600 space-y-0.5 max-h-72 overflow-y-auto">
              <For each={logs()}>
                {(line) => <p class="whitespace-pre-wrap break-all">{line}</p>}
              </For>
              <Show when={logs().length === 0}>
                <p class="text-gray-400">点击「发起验证」开始</p>
              </Show>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
