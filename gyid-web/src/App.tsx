import { Router, Route, A } from "@solidjs/router";
import type { ParentProps } from "solid-js";
import { Show } from "solid-js";
import Identity from "./pages/Identity";
import Collect from "./pages/Collect";
import Import from "./pages/Import";
import Verify from "./pages/Verify";
import Certificates from "./pages/Certificates";
import Explorer from "./pages/Explorer";
import { identityStore, lock } from "./stores/identity";

// 顶部导航：6 个主入口（Identity / Collect / Import / Verify / Certificates / Explorer）
// 注：@solidjs/router v0.15 的链接组件叫 A（非 Link）
const NavLink = (props: { href: string; label: string }) => (
  <A
    href={props.href}
    class="text-sm font-medium text-gray-600 hover:text-gray-900 transition-colors"
    activeClass="text-gray-900"
  >
    {props.label}
  </A>
);

const short = (h: string) => `${h.slice(0, 8)}…${h.slice(-6)}`;

// 整体布局：顶部 nav + 居中内容区
function Layout(props: ParentProps) {
  return (
    <div class="min-h-full flex flex-col">
      <nav class="bg-white border-b px-4 py-3 flex items-center gap-4">
        <span class="font-bold text-gray-900 mr-2">GyID</span>
        <NavLink href="/" label="Identity" />
        <NavLink href="/collect" label="Collect" />
        <NavLink href="/import" label="Import" />
        <NavLink href="/verify" label="Verify" />
        <NavLink href="/certificates" label="Certificates" />
        <NavLink href="/explorer" label="Explorer" />
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
    </div>
  );
}

export default function App() {
  return (
    <Router root={Layout}>
      <Route path="/" component={Identity} />
      <Route path="/collect" component={Collect} />
      <Route path="/import" component={Import} />
      <Route path="/verify" component={Verify} />
      <Route path="/certificates" component={Certificates} />
      <Route path="/explorer" component={Explorer} />
    </Router>
  );
}
