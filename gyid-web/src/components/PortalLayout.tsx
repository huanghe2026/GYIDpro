// TRIP 门户布局：公开介绍站的顶部导航 + 页脚（无解锁门槛、不依赖 wasm）。
import type { ParentProps } from "solid-js";
import { A } from "@solidjs/router";
import LangSwitch from "./LangSwitch";
import { dict, t } from "../i18n";

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
  const d = dict;
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
          <NavLink href="/" label={t("nav.home")} />
          <NavLink href="/protocol" label={t("nav.protocol")} />
          <NavLink href="/architecture" label={t("nav.architecture")} />
          <LangSwitch variant="dark" />
          <A
            href="/console"
            class="text-sm font-semibold rounded-md bg-white/10 hover:bg-white/20 px-3 py-1.5 transition-colors"
          >
            {t("nav.console")}
          </A>
        </div>
      </nav>
      <main class="flex-1">{props.children}</main>
      <footer class="border-t border-white/10 px-4 sm:px-8 py-8 text-sm text-slate-400">
        <div class="max-w-6xl mx-auto grid gap-6 sm:grid-cols-3">
          <div>
            <p class="font-semibold text-slate-200 mb-2">{d().footer.implTitle}</p>
            <p class="text-xs leading-relaxed">
              {d().footer.implBodyPre}
              {" "}<code class="text-slate-300">draft-ayerbe-trip-protocol-04</code>{" "}
              {d().footer.implBodyPost}
            </p>
          </div>
          <div>
            <p class="font-semibold text-slate-200 mb-2">{t("footer.resources")}</p>
            <ul class="space-y-1 text-xs">
              <li><A href="/protocol" class="hover:text-white">{t("footer.protocolPrimer")}</A></li>
              <li><A href="/architecture" class="hover:text-white">{t("footer.archApi")}</A></li>
              <li>
                <A href="/console/explorer" class="hover:text-white">
                  {t("footer.publicExplorer")}
                </A>
              </li>
            </ul>
          </div>
          <div>
            <p class="font-semibold text-slate-200 mb-2">{t("footer.statusTitle")}</p>
            <p class="text-xs leading-relaxed">
              {d().footer.statusBody}
            </p>
          </div>
        </div>
      </footer>
    </div>
  );
}
