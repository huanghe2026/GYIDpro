// 解锁门禁：需要已解锁身份的页面用 <UnlockGate>...</UnlockGate> 包裹。
// 未解锁时渲染内联的解锁表单（无账户则引导去 Identity 页创建）。

import { createSignal, Show, type ParentProps } from "solid-js";
import { A } from "@solidjs/router";
import { activeAccount, identityStore, unlock } from "../stores/identity";

export default function UnlockGate(props: ParentProps) {
  const [passphrase, setPassphrase] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);

  const submit = async () => {
    const acc = activeAccount();
    if (!acc) return;
    setError(null);
    try {
      await unlock(acc.id, passphrase());
      setPassphrase("");
    } catch {
      setError("解锁失败，请检查 passphrase");
    }
  };

  return (
    <Show
      when={identityStore.session}
      fallback={
        <div class="bg-white rounded-lg shadow p-6 max-w-md">
          <h2 class="text-lg font-semibold mb-1">需要解锁身份</h2>
          <Show
            when={activeAccount()}
            fallback={
              <p class="text-gray-600 text-sm">
                还没有身份。先到{" "}
                <A href="/console" class="text-blue-600 hover:underline">
                  Identity 页
                </A>{" "}
                创建一个。
              </p>
            }
          >
            <p class="text-gray-600 text-sm mb-4">
              当前账户：<span class="font-medium">{activeAccount()?.label}</span>
              ，输入 passphrase 解锁（seed 仅存在于内存，刷新页面需重新解锁）。
            </p>
            <input
              type="password"
              class="w-full border rounded px-3 py-2 mb-3 text-sm"
              placeholder="passphrase"
              value={passphrase()}
              onInput={(e) => setPassphrase(e.currentTarget.value)}
              onKeyDown={(e) => e.key === "Enter" && submit()}
            />
            <Show when={error()}>
              <p class="text-red-600 text-sm mb-2">{error()}</p>
            </Show>
            <button
              class="w-full bg-blue-600 text-white rounded py-2 text-sm font-medium hover:bg-blue-700 disabled:opacity-50"
              disabled={identityStore.busy || !passphrase()}
              onClick={submit}
            >
              {identityStore.busy ? "解锁中…" : "解锁"}
            </button>
          </Show>
        </div>
      }
    >
      {props.children}
    </Show>
  );
}
