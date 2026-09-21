// 控制台布局：GyID Web App 顶部导航（Identity / Collect / Import / Verify /
// Certificates / Explorer）+ 解锁状态 + 返回 TRIP 门户入口。
import type { ParentProps } from "solid-js";
import { Show } from "solid-js";
import { A } from "@solidjs/router";
import { identityStore, lock } from "../stores/identity";

const short = (h: string) => `${h.slice(0, 8)}…${h.slice(-6)}`;

// 注：@solidjs/router v0.15 的链接组件叫 A（非 Link）
const NavLink = (props: { href: string; label: string }) => (
  <A
    href={props.href}
    class="text-sm font-medium text-gray-600 hover:text-gray-900 transition-colors"
    activeClass="text-gray-900"
    end={props.href === "/console"}
  >
    {props.label}
  </A>
);

export default function ConsoleLayout(props: ParentProps) {
  return (
    <div class="min-h-full flex flex-col">
      <nav class="bg-white border-b px-4 py-3 flex items-center gap-4 flex-wrap">
        <A href="/" class="font-bold text-gray-900 mr-1">
          GyID
        </A>
        <span class="text-xs text-gray-400 hidden sm:inline">TRIP 控制台</span>
        <span class="text-gray-200">|</span>
        <NavLink href="/console" label="Identity" />
        <NavLink href="/console/collect" label="Collect" />
        <NavLink href="/console/import" label="Import" />
        <NavLink href="/console/verify" label="Verify" />
        <NavLink href="/console/certificates" label="Certificates" />
        <NavLink href="/console/explorer" label="Explorer" />
        <div class="ml-auto flex items-center gap-2 text-xs">
          <Show
            when={identityStore.session}
            fallback={<span class="text-gray-400">未解锁</span>}
          >
            <span class="text-green-700">●</span>
            <code class="text-gray-600">
              {short(identityStore.session!.pubkeyHex)}
            </code>
            <button
              class="border rounded px-2 py-0.5 text-gray-600 hover:bg-gray-50"
              onClick={lock}
            >
              锁定
            </button>
          </Show>
        </div>
      </nav>
      <main class="flex-1 max-w-5xl mx-auto p-6 w-full">{props.children}</main>
      <footer class="border-t bg-white py-3 text-center text-xs text-gray-400">
        <A href="/" class="hover:text-gray-600">
          ← 返回 TRIP 门户
        </A>
      </footer>
    </div>
  );
}
