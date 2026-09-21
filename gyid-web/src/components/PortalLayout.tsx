// TRIP 门户布局：公开介绍站的顶部导航 + 页脚（无解锁门槛、不依赖 wasm）。
import type { ParentProps } from "solid-js";
import { A } from "@solidjs/router";

const NavLink = (props: { href: string; label: string }) => (
  <A
    href={props.href}
    class="text-sm font-medium text-slate-300 hover:text-white transition-colors"
    activeClass="text-white"
    end={props.href === "/"}
  >
    {props.label}
  </A>
);

export default function PortalLayout(props: ParentProps) {
  return (
    <div class="min-h-full flex flex-col bg-slate-950 text-slate-100">
      <nav class="border-b border-white/10 px-4 sm:px-8 py-4 flex items-center gap-6">
        <A href="/" class="flex items-center gap-2 font-bold">
          <span class="inline-flex h-7 w-7 items-center justify-center rounded-md bg-gradient-to-br from-emerald-400 to-cyan-500 text-slate-950 text-sm font-black">
            T
          </span>
          <span>TRIP<span class="text-slate-500 font-normal mx-1">·</span><span class="text-slate-400 font-semibold">GyID</span></span>
        </A>
        <div class="ml-auto flex items-center gap-5">
          <NavLink href="/" label="首页" />
          <NavLink href="/protocol" label="协议" />
          <NavLink href="/architecture" label="架构" />
          <A
            href="/console"
            class="text-sm font-semibold rounded-md bg-white/10 hover:bg-white/20 px-3 py-1.5 transition-colors"
          >
            控制台
          </A>
        </div>
      </nav>
      <main class="flex-1">{props.children}</main>
      <footer class="border-t border-white/10 px-4 sm:px-8 py-8 text-sm text-slate-400">
        <div class="max-w-6xl mx-auto grid gap-6 sm:grid-cols-3">
          <div>
            <p class="font-semibold text-slate-200 mb-2">GyID · TRIP 实现</p>
            <p class="text-xs leading-relaxed">
              基于 IETF 互联网草案
              {" "}<code class="text-slate-300">draft-ayerbe-trip-protocol-04</code>{" "}
              的轨迹身份开源实现：面包屑证明链、主动验证与人类存在证明（PoH）。
            </p>
          </div>
          <div>
            <p class="font-semibold text-slate-200 mb-2">资源</p>
            <ul class="space-y-1 text-xs">
              <li><A href="/protocol" class="hover:text-white">协议科普</A></li>
              <li><A href="/architecture" class="hover:text-white">系统架构与 Verifier API</A></li>
              <li>
                <A href="/console/explorer" class="hover:text-white">
                  公开身份浏览器
                </A>
              </li>
            </ul>
          </div>
          <div>
            <p class="font-semibold text-slate-200 mb-2">状态声明</p>
            <p class="text-xs leading-relaxed">
              TRIP 仍为 IETF Internet-Draft（非正式 RFC），本项目仅供研究与实验，
              不应用于生产安全决策。
            </p>
          </div>
        </div>
      </footer>
    </div>
  );
}
