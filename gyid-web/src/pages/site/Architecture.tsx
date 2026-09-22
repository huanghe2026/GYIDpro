// 架构页：GyID 的 crate 地图、数据流、Verifier API 与开发者快速开始。
import { A } from "@solidjs/router";
import { dict, t } from "../../i18n";

const methodColor: Record<string, string> = {
  GET: "text-cyan-300 border-cyan-400/30 bg-cyan-400/10",
  POST: "text-emerald-300 border-emerald-400/30 bg-emerald-400/10",
};

const Code = (props: { children: string }) => (
  <pre class="rounded-lg border border-white/10 bg-black/40 p-4 overflow-x-auto text-xs leading-relaxed text-slate-300 font-mono">
    {props.children}
  </pre>
);

function H2(props: { eyebrow: string; title: string; desc?: string }) {
  return (
    <div class="mb-8">
      <p class="text-xs font-mono text-emerald-400 mb-1">{props.eyebrow}</p>
      <h2 class="text-2xl sm:text-3xl font-bold">{props.title}</h2>
      {props.desc && <p class="mt-2 text-sm text-slate-400">{props.desc}</p>}
    </div>
  );
}

export default function Architecture() {
  const d = dict;
  const a = () => d().architecture;
  return (
    <div class="px-4 sm:px-8 py-14">
      <div class="max-w-5xl mx-auto">
        <p class="text-xs font-mono text-emerald-400 mb-3">{a().header.eyebrow}</p>
        <h1 class="text-3xl sm:text-5xl font-black tracking-tight">{a().header.title}</h1>
        <p class="mt-5 text-slate-300 leading-relaxed max-w-3xl">
          {a().header.intro}
        </p>

        {/* 数据流 */}
        <section class="mt-14">
          <H2 eyebrow={t("architecture.dataflow.eyebrow")} title={t("architecture.dataflow.title")} />
          <div class="rounded-xl border border-white/10 bg-white/[0.03] p-6">
            <ol class="space-y-3 text-sm text-slate-300">
              {a().dataflow.items.map((step, i) => (
                <li class="flex gap-3">
                  <span class="font-mono text-emerald-400 shrink-0">{i + 1}.</span>
                  <span class="text-slate-400 leading-relaxed">{step}</span>
                </li>
              ))}
            </ol>
          </div>
        </section>

        {/* Crate 地图 */}
        <section class="mt-16">
          <H2 eyebrow={t("architecture.modules.eyebrow")} title={t("architecture.modules.title")} />
          <div class="grid gap-4 sm:grid-cols-2">
            {a().modules.crates.map((c) => (
              <div class="rounded-xl border border-white/10 bg-white/[0.03] p-5">
                <div class="flex items-center gap-2 flex-wrap mb-2">
                  <code class="font-bold text-emerald-300">{c.name}</code>
                  <span class="text-[11px] rounded-full border border-white/15 text-slate-400 px-2 py-0.5">
                    {c.lang}
                  </span>
                  <span class="text-[11px] text-slate-500">{c.role}</span>
                </div>
                <p class="text-xs text-slate-400 leading-relaxed">{c.body}</p>
              </div>
            ))}
          </div>
        </section>

        {/* API */}
        <section class="mt-16">
          <H2
            eyebrow={t("architecture.api.eyebrow")}
            title={t("architecture.api.title")}
            desc={a().api.desc}
          />
          <div class="rounded-xl border border-white/10 overflow-hidden">
            {a().api.endpoints.map((ep, i) => (
              <div
                class={`grid sm:grid-cols-[70px_230px_1fr] gap-2 sm:gap-4 px-4 py-3 text-sm items-start ${
                  i % 2 === 0 ? "bg-white/[0.03]" : "bg-transparent"
                }`}
              >
                <span
                  class={`justify-self-start text-[11px] font-mono font-bold rounded border px-1.5 py-0.5 ${methodColor[ep.m]}`}
                >
                  {ep.m}
                </span>
                <code class="text-xs text-slate-200 break-all">{ep.path}</code>
                <span class="text-xs text-slate-400 leading-relaxed">{ep.desc}</span>
              </div>
            ))}
          </div>
        </section>

        {/* 开发者快速开始 */}
        <section class="mt-16">
          <H2 eyebrow={t("architecture.quickstart.eyebrow")} title={t("architecture.quickstart.title")} />
          <div class="space-y-6">
            {a().quickstart.steps.map((s) => (
              <div>
                <p class="text-sm font-semibold mb-2 text-slate-200">{s.title}</p>
                <Code>{s.code}</Code>
              </div>
            ))}
          </div>
          <div class="mt-8 rounded-lg border border-white/10 bg-white/[0.03] px-4 py-3 text-sm text-slate-400">
            {a().quickstart.ctaPre}
            <A href="/console" class="text-emerald-400 hover:text-emerald-300 font-semibold">
              {a().quickstart.ctaLink}
            </A>
          </div>
        </section>
      </div>
    </div>
  );
}
