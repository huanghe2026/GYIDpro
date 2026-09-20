// Identity 页：多账户管理。
//
// - 新建身份：wasm generate_keypair → passphrase（PBKDF2 250k + AES-256-GCM 加密 seed）
//   → 加密 JSON blob 落 localStorage；seed 明文只在内存会话中
// - 解锁 / 切换 / 锁定 / 删除
// - 已解锁时调 GET /v1/identity/:pubkey 展示链上统计（404 = 还没上传过面包屑）

import { createResource, createSignal, For, Show } from "solid-js";
import {
  activeAccount,
  createIdentity,
  identityStore,
  lock,
  removeAccount,
  selectAccount,
  unlock,
} from "../stores/identity";
import { fetchIdentity } from "../lib/verifier";

const short = (h: string) =>
  h.length > 20 ? `${h.slice(0, 10)}…${h.slice(-8)}` : h;
const formatMs = (ms: number) => new Date(ms).toLocaleString();
const formatTs = (sec: number) => new Date(sec * 1000).toLocaleString();

export default function Identity() {
  // ----- 新建身份表单 -----
  const [showCreate, setShowCreate] = createSignal(false);
  const [label, setLabel] = createSignal("");
  const [pass, setPass] = createSignal("");
  const [pass2, setPass2] = createSignal("");
  const [formErr, setFormErr] = createSignal<string | null>(null);

  // ----- 解锁表单（当前账户未解锁时） -----
  const [unlockPass, setUnlockPass] = createSignal("");
  const [unlockErr, setUnlockErr] = createSignal<string | null>(null);

  const [copied, setCopied] = createSignal(false);

  // 已解锁身份的链上统计；session 变化时自动重取，null pubkey 时不请求
  const [stats, statsActions] = createResource(
    () => identityStore.session?.pubkeyHex ?? null,
    async (pk) => (pk ? fetchIdentity(pk) : null),
  );

  const submitCreate = async () => {
    setFormErr(null);
    if (pass().length < 6) {
      setFormErr("passphrase 至少 6 位");
      return;
    }
    if (pass() !== pass2()) {
      setFormErr("两次输入的 passphrase 不一致");
      return;
    }
    try {
      await createIdentity(label(), pass());
      setShowCreate(false);
      setLabel("");
      setPass("");
      setPass2("");
    } catch {
      setFormErr(identityStore.error ?? "创建失败");
    }
  };

  const submitUnlock = async () => {
    const acc = activeAccount();
    if (!acc) return;
    setUnlockErr(null);
    try {
      await unlock(acc.id, unlockPass());
      setUnlockPass("");
    } catch {
      setUnlockErr("解锁失败，请检查 passphrase");
    }
  };

  const copyPubkey = async () => {
    const pk = identityStore.session?.pubkeyHex;
    if (!pk) return;
    await navigator.clipboard.writeText(pk);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-bold">Identity</h1>
        <button
          class="bg-blue-600 text-white text-sm rounded px-3 py-2 hover:bg-blue-700"
          onClick={() => setShowCreate((v) => !v)}
        >
          {showCreate() ? "取消" : "+ 新建身份"}
        </button>
      </div>

      {/* 新建身份表单 */}
      <Show when={showCreate()}>
        <div class="bg-white rounded-lg shadow p-6 space-y-3 max-w-lg">
          <h2 class="font-semibold">新建身份</h2>
          <input
            class="w-full border rounded px-3 py-2 text-sm"
            placeholder="标签（可选，如 phone / laptop）"
            value={label()}
            onInput={(e) => setLabel(e.currentTarget.value)}
          />
          <input
            type="password"
            class="w-full border rounded px-3 py-2 text-sm"
            placeholder="passphrase（至少 6 位）"
            value={pass()}
            onInput={(e) => setPass(e.currentTarget.value)}
          />
          <input
            type="password"
            class="w-full border rounded px-3 py-2 text-sm"
            placeholder="再输一遍 passphrase"
            value={pass2()}
            onInput={(e) => setPass2(e.currentTarget.value)}
            onKeyDown={(e) => e.key === "Enter" && submitCreate()}
          />
          <Show when={formErr()}>
            <p class="text-red-600 text-sm">{formErr()}</p>
          </Show>
          <button
            class="bg-blue-600 text-white text-sm rounded px-4 py-2 hover:bg-blue-700 disabled:opacity-50"
            disabled={identityStore.busy}
            onClick={submitCreate}
          >
            {identityStore.busy ? "生成中…" : "生成并加密保存"}
          </button>
        </div>
      </Show>

      {/* 当前选中账户：已解锁面板 */}
      <Show when={identityStore.session}>
        {(session) => (
          <div class="bg-white rounded-lg shadow p-6 space-y-4">
            <div class="flex items-start justify-between gap-4">
              <div>
                <h2 class="font-semibold text-lg">
                  {activeAccount()?.label ?? "default"}
                </h2>
                <p class="text-xs text-green-700 mt-0.5">● 已解锁（seed 仅在内存）</p>
              </div>
              <button
                class="text-sm border rounded px-3 py-1.5 hover:bg-gray-50"
                onClick={lock}
              >
                锁定
              </button>
            </div>

            <div>
              <div class="text-xs text-gray-500 mb-1">Attester 公钥（pubkey hex）</div>
              <div class="flex items-center gap-2">
                <code class="text-xs bg-gray-100 rounded px-2 py-1.5 break-all flex-1">
                  {session().pubkeyHex}
                </code>
                <button
                  class="text-sm border rounded px-2.5 py-1.5 hover:bg-gray-50 shrink-0"
                  onClick={copyPubkey}
                >
                  {copied() ? "已复制" : "复制"}
                </button>
              </div>
            </div>

            {/* 链上统计：GET /v1/identity/:hex */}
            <div>
              <div class="flex items-center justify-between mb-2">
                <div class="text-xs text-gray-500">Verifier 链上状态</div>
                <button
                  class="text-xs text-blue-600 hover:underline"
                  onClick={() => statsActions.refetch()}
                >
                  刷新
                </button>
              </div>
              <Show
                when={!stats.loading}
                fallback={<p class="text-sm text-gray-500">加载中…</p>}
              >
                <Show when={stats.error}>
                  <p class="text-sm text-red-600">
                    无法连接 Verifier：{stats.error?.message}（确认 trip-server 已启动）
                  </p>
                </Show>
                <Show when={!stats.error && stats()}>
                  {(info) => (
                    <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
                      <Stat label="面包屑总数" value={String(info().breadcrumb_count)} />
                      <Stat label="唯一 H3 cell" value={String(info().unique_cells)} />
                      <Stat label="链头 hash" value={short(info().chain_head)} mono />
                      <Stat label="最后时间戳" value={formatTs(info().last_ts)} />
                    </div>
                  )}
                </Show>
                <Show when={!stats.error && stats() === null}>
                  <p class="text-sm text-gray-500">
                    Verifier 上还没有这个身份的证据 —— 去 Collect 页采集并上传第一组面包屑。
                  </p>
                </Show>
              </Show>
            </div>
          </div>
        )}
      </Show>

      {/* 当前账户已选中但未解锁：解锁表单 */}
      <Show
        when={!identityStore.session && activeAccount()}
      >
        <div class="bg-white rounded-lg shadow p-6 max-w-lg space-y-3">
          <h2 class="font-semibold">
            解锁「{activeAccount()?.label}」
          </h2>
          <input
            type="password"
            class="w-full border rounded px-3 py-2 text-sm"
            placeholder="passphrase"
            value={unlockPass()}
            onInput={(e) => setUnlockPass(e.currentTarget.value)}
            onKeyDown={(e) => e.key === "Enter" && submitUnlock()}
          />
          <Show when={unlockErr()}>
            <p class="text-red-600 text-sm">{unlockErr()}</p>
          </Show>
          <button
            class="bg-blue-600 text-white text-sm rounded px-4 py-2 hover:bg-blue-700 disabled:opacity-50"
            disabled={identityStore.busy || !unlockPass()}
            onClick={submitUnlock}
          >
            {identityStore.busy ? "解锁中…" : "解锁"}
          </button>
        </div>
      </Show>

      {/* 全部本地账户 */}
      <div>
        <h2 class="font-semibold mb-3 text-sm text-gray-600">
          本地账户（{identityStore.accounts.length}）
        </h2>
        <Show
          when={identityStore.accounts.length > 0}
          fallback={
            <p class="text-sm text-gray-500 bg-white rounded-lg shadow p-6">
              暂无账户，点击右上角「+ 新建身份」开始。
            </p>
          }
        >
          <div class="grid gap-3 md:grid-cols-2">
            <For each={identityStore.accounts}>
              {(acc) => (
                <div
                  class="bg-white rounded-lg shadow p-4 flex items-center justify-between gap-3"
                  classList={{ "ring-2 ring-blue-500": acc.id === identityStore.activeId }}
                >
                  <div class="min-w-0">
                    <div class="font-medium text-sm truncate">{acc.label}</div>
                    <div class="text-xs text-gray-500">
                      创建于 {formatMs(acc.createdAt)}
                    </div>
                    <Show when={acc.id === identityStore.session?.id}>
                      <div class="text-xs text-green-700 mt-0.5">已解锁</div>
                    </Show>
                  </div>
                  <div class="flex gap-1.5 shrink-0">
                    <Show when={acc.id !== identityStore.activeId}>
                      <button
                        class="text-xs border rounded px-2 py-1 hover:bg-gray-50"
                        onClick={() => selectAccount(acc.id)}
                      >
                        切换
                      </button>
                    </Show>
                    <button
                      class="text-xs border rounded px-2 py-1 text-red-600 hover:bg-red-50"
                      onClick={() => {
                        if (
                          confirm(
                            `删除账户「${acc.label}」？加密记录将从本机移除（不可恢复）。`,
                          )
                        ) {
                          removeAccount(acc.id);
                        }
                      }}
                    >
                      删除
                    </button>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>
    </div>
  );
}

function Stat(props: { label: string; value: string; mono?: boolean }) {
  return (
    <div class="border rounded-lg px-3 py-2">
      <div class="text-xs text-gray-500">{props.label}</div>
      <div
        class="text-sm font-medium mt-0.5 truncate"
        classList={{ "font-mono text-xs": props.mono }}
        title={props.value}
      >
        {props.value}
      </div>
    </div>
  );
}
