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
import {
  explorerAddressUrl,
  fetchChainIdentity,
  registryAddress,
  registryConfigured,
} from "../lib/registry";
import { fmtDateTime, t } from "../i18n";

const short = (h: string) =>
  h.length > 20 ? `${h.slice(0, 10)}…${h.slice(-8)}` : h;

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

  // GeoTITRegistry 链上登记状态（仅在配置了合约地址时启用）
  const [chain, chainActions] = createResource(
    () =>
      registryConfigured() && identityStore.session?.pubkeyHex
        ? identityStore.session.pubkeyHex
        : null,
    async (pk) => fetchChainIdentity(pk),
  );

  const submitCreate = async () => {
    setFormErr(null);
    if (pass().length < 6) {
      setFormErr(t("identity.passTooShort"));
      return;
    }
    if (pass() !== pass2()) {
      setFormErr(t("identity.passMismatch"));
      return;
    }
    try {
      await createIdentity(label(), pass());
      setShowCreate(false);
      setLabel("");
      setPass("");
      setPass2("");
    } catch {
      setFormErr(identityStore.error ?? t("identity.createFail"));
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
      setUnlockErr(t("unlock.failed"));
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
          {showCreate() ? t("common.cancel") : t("identity.newBtn")}
        </button>
      </div>

      {/* 新建身份表单 */}
      <Show when={showCreate()}>
        <div class="bg-white rounded-lg shadow p-6 space-y-3 max-w-lg">
          <h2 class="font-semibold">{t("identity.createTitle")}</h2>
          <input
            class="w-full border rounded px-3 py-2 text-sm"
            placeholder={t("identity.labelPlaceholder")}
            value={label()}
            onInput={(e) => setLabel(e.currentTarget.value)}
          />
          <input
            type="password"
            class="w-full border rounded px-3 py-2 text-sm"
            placeholder={t("identity.passPlaceholder")}
            value={pass()}
            onInput={(e) => setPass(e.currentTarget.value)}
          />
          <input
            type="password"
            class="w-full border rounded px-3 py-2 text-sm"
            placeholder={t("identity.pass2Placeholder")}
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
            {identityStore.busy ? t("identity.generating") : t("identity.createSubmit")}
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
                <p class="text-xs text-green-700 mt-0.5">{t("identity.unlockedTag")}</p>
              </div>
              <button
                class="text-sm border rounded px-3 py-1.5 hover:bg-gray-50"
                onClick={lock}
              >
                {t("console.lock")}
              </button>
            </div>

            <div>
              <div class="text-xs text-gray-500 mb-1">{t("identity.pubkeyLabel")}</div>
              <div class="flex items-center gap-2">
                <code class="text-xs bg-gray-100 rounded px-2 py-1.5 break-all flex-1">
                  {session().pubkeyHex}
                </code>
                <button
                  class="text-sm border rounded px-2.5 py-1.5 hover:bg-gray-50 shrink-0"
                  onClick={copyPubkey}
                >
                  {copied() ? t("common.copied") : t("common.copy")}
                </button>
              </div>
            </div>

            {/* 链上统计：GET /v1/identity/:hex */}
            <div>
              <div class="flex items-center justify-between mb-2">
                <div class="text-xs text-gray-500">{t("identity.chainStatus")}</div>
                <button
                  class="text-xs text-blue-600 hover:underline"
                  onClick={() => statsActions.refetch()}
                >
                  {t("common.refresh")}
                </button>
              </div>
              <Show
                when={!stats.loading}
                fallback={<p class="text-sm text-gray-500">{t("common.loading")}</p>}
              >
                <Show when={stats.error}>
                  <p class="text-sm text-red-600">
                    {t("identity.connFail", { msg: stats.error?.message ?? "" })}
                  </p>
                </Show>
                <Show when={!stats.error && stats()}>
                  {(info) => (
                    <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
                      <Stat label={t("identity.statCount")} value={String(info().breadcrumb_count)} />
                      <Stat label={t("identity.statCells")} value={String(info().unique_cells)} />
                      <Stat label={t("identity.statHead")} value={short(info().chain_head)} mono />
                      <Stat label={t("identity.statLast")} value={fmtDateTime(info().last_ts * 1000)} />
                    </div>
                  )}
                </Show>
                <Show when={!stats.error && stats() === null}>
                  <p class="text-sm text-gray-500">
                    {t("identity.noEvidence")}
                  </p>
                </Show>
              </Show>
            </div>

            {/* GeoTITRegistry 链上登记（配置了 VITE_REGISTRY_ADDRESS 才显示） */}
            <Show when={registryConfigured()}>
              <div class="border-t pt-4">
                <div class="flex items-center justify-between mb-2">
                  <div class="text-xs text-gray-500">
                    {t("identity.onchain.title")}
                  </div>
                  <button
                    class="text-xs text-blue-600 hover:underline"
                    onClick={() => chainActions.refetch()}
                  >
                    {t("common.refresh")}
                  </button>
                </div>
                <Show
                  when={!chain.loading}
                  fallback={
                    <p class="text-sm text-gray-500">
                      {t("identity.onchain.loading")}
                    </p>
                  }
                >
                  <Show when={chain.error}>
                    <p class="text-sm text-red-600">
                      {t("identity.onchain.error")}
                    </p>
                  </Show>
                  <Show when={!chain.error && chain() && !chain()!.registered}>
                    <p class="text-sm text-gray-500">
                      {t("identity.onchain.unregistered")}
                    </p>
                  </Show>
                  <Show when={!chain.error && chain()?.registered}>
                    <div>
                      <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
                        <Stat
                          label={t("identity.onchain.registeredAt")}
                          value={fmtDateTime(chain()!.registeredAt * 1000)}
                        />
                        <Stat
                          label={t("identity.onchain.epochs")}
                          value={String(chain()!.epochCount)}
                        />
                        <Stat
                          label={t("identity.onchain.cells")}
                          value={String(chain()!.lastUniqueCells)}
                        />
                        <Stat
                          label={t("identity.onchain.handle")}
                          value={chain()!.handle || t("identity.onchain.handleNone")}
                        />
                      </div>
                      <a
                        class="inline-block mt-2 text-xs text-blue-600 hover:underline"
                        href={explorerAddressUrl(registryAddress())}
                        target="_blank"
                        rel="noreferrer"
                      >
                        {t("identity.onchain.view")} ↗
                      </a>
                    </div>
                  </Show>
                </Show>
              </div>
            </Show>
          </div>
        )}
      </Show>

      {/* 当前账户已选中但未解锁：解锁表单 */}
      <Show
        when={!identityStore.session && activeAccount()}
      >
        <div class="bg-white rounded-lg shadow p-6 max-w-lg space-y-3">
          <h2 class="font-semibold">
            {t("identity.unlockTitle", { label: activeAccount()?.label ?? "" })}
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
            {identityStore.busy ? t("identity.unlocking") : t("unlock.btn")}
          </button>
        </div>
      </Show>

      {/* 全部本地账户 */}
      <div>
        <h2 class="font-semibold mb-3 text-sm text-gray-600">
          {t("identity.localAccounts", { n: identityStore.accounts.length })}
        </h2>
        <Show
          when={identityStore.accounts.length > 0}
          fallback={
            <p class="text-sm text-gray-500 bg-white rounded-lg shadow p-6">
              {t("identity.noAccounts")}
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
                      {t("identity.createdAt", { date: fmtDateTime(acc.createdAt) })}
                    </div>
                    <Show when={acc.id === identityStore.session?.id}>
                      <div class="text-xs text-green-700 mt-0.5">{t("identity.unlocked")}</div>
                    </Show>
                  </div>
                  <div class="flex gap-1.5 shrink-0">
                    <Show when={acc.id !== identityStore.activeId}>
                      <button
                        class="text-xs border rounded px-2 py-1 hover:bg-gray-50"
                        onClick={() => selectAccount(acc.id)}
                      >
                        {t("identity.switch")}
                      </button>
                    </Show>
                    <button
                      class="text-xs border rounded px-2 py-1 text-red-600 hover:bg-red-50"
                      onClick={() => {
                        if (confirm(t("identity.deleteConfirm", { label: acc.label }))) {
                          removeAccount(acc.id);
                        }
                      }}
                    >
                      {t("common.delete")}
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
