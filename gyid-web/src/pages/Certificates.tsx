// Certificates 页：RP 视角的 PoH 证书列表（draft-04 §10）。
//
// 流程：GET /v1/pohs?attester=<pubkey> → 逐个 POST /v1/poh 取 CBOR →
// wasm verify_poh 本地校验 → 状态徽章（Fresh+PASS 绿 / Stale+FAIL 红）。
//
// 注：列表视角拿不到当年 RP 的原始 rp_nonce，因此两段式校验——先用全零
// nonce 解析出证书自带 nonce，再用它复验。nonce 自比恒通过，fresh 于是
// 只反映「Verifier 签名有效 + 未过期」；绑定 nonce 的强校验在 Verify 页做。

import { createResource, For, Show } from "solid-js";
import UnlockGate from "../components/UnlockGate";
import { identityStore } from "../stores/identity";
import {
  bytesToHex,
  fetchPoh,
  fetchVerifierPubkeyHex,
  listPohs,
} from "../lib/verifier";
import { loadWasm, type PohInfoJs } from "../lib/wasm";

/** 策略门槛（与 trip-server Config::default 对齐） */
const MIN_CONFIDENCE = 0.1;
const MIN_TRUST = 20.0;

const ZERO_NONCE = "00".repeat(16);

const short = (h: string) =>
  h.length > 20 ? `${h.slice(0, 10)}…${h.slice(-8)}` : h;

interface CertEntry {
  challengeId: string;
  info?: PohInfoJs;
  error?: string;
}

/** 徽章：kind + 文案 */
function badge(info: PohInfoJs): { cls: string; text: string } {
  if (info.is_trusted) return { cls: "bg-green-100 text-green-800", text: "Fresh + PASS" };
  if (info.fresh && !info.policy_pass)
    return { cls: "bg-amber-100 text-amber-800", text: "Fresh，策略未过" };
  if (!info.fresh && info.policy_pass)
    return { cls: "bg-amber-100 text-amber-800", text: "已过期" };
  return { cls: "bg-red-100 text-red-800", text: "Stale / FAIL" };
}

export default function Certificates() {
  return (
    <UnlockGate>
      <CertificatesInner />
    </UnlockGate>
  );
}

function CertificatesInner() {
  const session = () => identityStore.session;

  const [certs, certsActions] = createResource(
    () => session()?.pubkeyHex ?? null,
    async (pubkey) => {
      const wasm = await loadWasm();
      const vk = await fetchVerifierPubkeyHex();
      const list = await listPohs(pubkey);
      if (!list || list.challenge_ids.length === 0) return [] as CertEntry[];
      const now = BigInt(Math.floor(Date.now() / 1000));
      const out: CertEntry[] = [];
      // 列表按时间正序返回，倒序展示（最新的在最上）
      for (const id of [...list.challenge_ids].reverse()) {
        try {
          const r = await fetchPoh(id);
          if (!(r instanceof Uint8Array)) throw new Error("still pending");
          const hex = bytesToHex(r);
          // 第一遍：全零 nonce → 拿到证书自带 nonce（fresh 会因 nonce 不匹配为 false）
          const probe = wasm.verify_poh(hex, vk, ZERO_NONCE, now, MIN_CONFIDENCE, MIN_TRUST);
          // 第二遍：用证书自己的 nonce 复验 → fresh 只反映签名 + 有效期
          const info = wasm.verify_poh(hex, vk, probe.nonce_hex, now, MIN_CONFIDENCE, MIN_TRUST);
          out.push({ challengeId: id, info });
        } catch (e) {
          out.push({ challengeId: id, error: String(e) });
        }
      }
      return out;
    },
  );

  const loading = () => certs.loading;

  return (
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold">Certificates</h1>
          <p class="text-xs text-gray-500 mt-0.5 font-mono">
            attester: {session() ? short(session()!.pubkeyHex) : "—"}
          </p>
        </div>
        <button
          class="border rounded px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50"
          disabled={loading()}
          onClick={() => void certsActions.refetch()}
        >
          {loading() ? "加载中…" : "刷新"}
        </button>
      </div>

      <Show when={!loading() && certs()?.length === 0}>
        <div class="bg-white rounded-lg shadow p-6 text-sm text-gray-500">
          该身份还没有 PoH 证书。先在 <b>Collect</b> 页采集并上传面包屑，再到{" "}
          <b>Verify</b> 页发起一次 Active Verification。
        </div>
      </Show>

      <Show when={certs() && certs()!.length > 0}>
        <div class="space-y-3">
          <For each={certs()}>
            {(entry) => (
              <Show
                when={entry.info}
                fallback={
                  <div class="bg-white rounded-lg shadow p-4 border-l-4 border-gray-300">
                    <p class="font-mono text-xs text-gray-600">
                      challenge_id: {entry.challengeId}
                    </p>
                    <p class="text-sm text-red-600 mt-1 break-all">
                      取回失败：{entry.error}
                    </p>
                  </div>
                }
              >
                <div class={`bg-white rounded-lg shadow p-4 border-l-4 ${entry.info!.is_trusted ? "border-green-500" : "border-red-400"}`}>
                  <div class="flex items-center gap-3 flex-wrap">
                    <span
                      class={`text-xs font-bold px-2 py-0.5 rounded ${badge(entry.info!).cls}`}
                    >
                      {badge(entry.info!).text}
                    </span>
                    <span class="font-mono text-xs text-gray-600">
                      {entry.challengeId}
                    </span>
                  </div>
                  <dl class="grid grid-cols-2 sm:grid-cols-4 gap-x-4 gap-y-1.5 text-sm mt-3">
                    <div>
                      <dt class="text-xs text-gray-500">签发</dt>
                      <dd>{new Date(entry.info!.issued_at * 1000).toLocaleString()}</dd>
                    </div>
                    <div>
                      <dt class="text-xs text-gray-500">有效期至</dt>
                      <dd>
                        {new Date((entry.info!.issued_at + entry.info!.validity_secs) * 1000).toLocaleTimeString()}
                      </dd>
                    </div>
                    <div>
                      <dt class="text-xs text-gray-500">面包屑 / cell</dt>
                      <dd class="font-mono">
                        {entry.info!.breadcrumb_count} / {entry.info!.unique_cells}
                      </dd>
                    </div>
                    <div>
                      <dt class="text-xs text-gray-500">trust / conf / α</dt>
                      <dd class="font-mono">
                        {entry.info!.trust.toFixed(3)} / {entry.info!.criticality_confidence.toFixed(3)} / {entry.info!.alpha.toFixed(3)}
                      </dd>
                    </div>
                  </dl>
                </div>
              </Show>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
