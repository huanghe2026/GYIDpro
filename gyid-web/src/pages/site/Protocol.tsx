// 协议科普页：TRIP draft-04 是什么。内容整理自 src/public/trip.md。
import { dict, t } from "../../i18n";

function H2(props: { eyebrow: string; title: string }) {
  return (
    <div class="mb-8">
      <p class="text-xs font-mono text-emerald-400 mb-1">{props.eyebrow}</p>
      <h2 class="text-2xl sm:text-3xl font-bold">{props.title}</h2>
    </div>
  );
}

export default function Protocol() {
  const d = dict;
  const p = () => d().protocol;
  return (
    <div class="px-4 sm:px-8 py-14">
      <div class="max-w-4xl mx-auto">
        {/* 头部 */}
        <p class="text-xs font-mono text-emerald-400 mb-3">
          {p().header.eyebrow}
        </p>
        <h1 class="text-3xl sm:text-5xl font-black tracking-tight">
          {p().header.title}
        </h1>
        <p class="mt-5 text-slate-300 leading-relaxed">
          {p().header.introPre}
          <span class="text-white">{p().header.introHl}</span>
          {p().header.introPost}
        </p>
        <div class="mt-6 rounded-lg border border-amber-400/30 bg-amber-400/10 px-4 py-3 text-sm text-amber-200">
          {p().header.noticePre}<b>{p().header.noticeBold}</b>{p().header.noticePost}
        </div>

        {/* 背景 */}
        <section class="mt-16">
          <H2 eyebrow={t("protocol.background.eyebrow")} title={t("protocol.background.title")} />
          <div class="space-y-4 text-sm text-slate-300 leading-relaxed">
            <p>
              {p().background.p1Pre}
              <span class="text-white">{p().background.p1Hl}</span>
            </p>
            <p>{p().background.p2}</p>
            <ul class="list-disc pl-5 space-y-2 text-slate-400">
              <li>{p().background.li1}</li>
              <li>
                {p().background.li2Pre}
                <span class="text-slate-200">{p().background.li2Hl}</span>
                {p().background.li2Post}
              </li>
            </ul>
          </div>
        </section>

        {/* 时间线 */}
        <section class="mt-16">
          <H2 eyebrow={t("protocol.timeline.eyebrow")} title={t("protocol.timeline.title")} />
          <div class="space-y-4">
            {p().timeline.items.map((ti) => (
              <div
                class={`rounded-xl border p-5 ${
                  ti.v === "-04"
                    ? "border-emerald-400/40 bg-emerald-400/5"
                    : "border-white/10 bg-white/[0.03]"
                }`}
              >
                <div class="flex items-baseline gap-3 flex-wrap">
                  <span class="font-mono font-black text-lg text-emerald-400">{ti.v}</span>
                  <span class="text-xs text-slate-500 font-mono">{ti.date}</span>
                  <span class="font-bold">{ti.title}</span>
                  {ti.v === "-04" && (
                    <span class="text-[11px] rounded-full bg-emerald-400/20 text-emerald-300 px-2 py-0.5">
                      {t("protocol.timeline.implementedBadge")}
                    </span>
                  )}
                </div>
                <p class="mt-2 text-sm text-slate-400 leading-relaxed">{ti.body}</p>
              </div>
            ))}
          </div>
        </section>

        {/* RATS 角色 */}
        <section class="mt-16">
          <H2 eyebrow={t("protocol.roles.eyebrow")} title={t("protocol.roles.title")} />
          <div class="grid gap-4 sm:grid-cols-3">
            {p().roles.items.map((r) => (
              <div class="rounded-xl border border-white/10 bg-white/[0.03] p-5">
                <p class="font-mono text-xs text-emerald-400">{r.en}</p>
                <p class="font-bold mt-1 mb-2">{r.cn}</p>
                <p class="text-xs text-slate-400 leading-relaxed">{r.body}</p>
              </div>
            ))}
          </div>
        </section>

        {/* 概念表 */}
        <section class="mt-16">
          <H2 eyebrow={t("protocol.glossary.eyebrow")} title={t("protocol.glossary.title")} />
          <dl class="rounded-xl border border-white/10 overflow-hidden">
            {p().glossary.items.map(([term, desc], i) => (
              <div
                class={`grid sm:grid-cols-[180px_1fr] gap-1 sm:gap-4 px-5 py-4 text-sm ${
                  i % 2 === 0 ? "bg-white/[0.03]" : "bg-transparent"
                }`}
              >
                <dt class="font-mono font-bold text-emerald-300">{term}</dt>
                <dd class="text-slate-400 leading-relaxed">{desc}</dd>
              </div>
            ))}
          </dl>
        </section>

        {/* 隐私 */}
        <section class="mt-16">
          <H2 eyebrow={t("protocol.privacy.eyebrow")} title={t("protocol.privacy.title")} />
          <ul class="list-disc pl-5 space-y-2 text-sm text-slate-400 leading-relaxed">
            {p().privacy.items.map((item) =>
              typeof item === "string" ? (
                <li>{item}</li>
              ) : (
                <li>
                  {item.pre}<span class="text-slate-200">{item.hl}</span>{item.post}
                </li>
              ),
            )}
          </ul>
        </section>

        {/* 局限 */}
        <section class="mt-16">
          <H2 eyebrow={t("protocol.limitations.eyebrow")} title={t("protocol.limitations.title")} />
          <ul class="list-disc pl-5 space-y-2 text-sm text-slate-400 leading-relaxed">
            {p().limitations.items.map((l) => (
              <li>{l}</li>
            ))}
          </ul>
        </section>

        {/* 场景 */}
        <section class="mt-16">
          <H2 eyebrow={t("protocol.usecases.eyebrow")} title={t("protocol.usecases.title")} />
          <div class="grid gap-3 sm:grid-cols-2">
            {p().usecases.items.map((u) => (
              <div class="rounded-lg border border-white/10 bg-white/[0.03] px-4 py-3 text-sm text-slate-300">
                {u}
              </div>
            ))}
          </div>
          <p class="mt-5 text-xs text-slate-500 leading-relaxed">
            {p().usecases.notePre}
            <span class="text-slate-300">{p().usecases.noteHl}</span>
            {p().usecases.notePost}
          </p>
        </section>
      </div>
    </div>
  );
}
