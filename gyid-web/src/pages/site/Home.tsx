// 首页：面向普通用户的 Geoyuan ID（GyID）落地页。
// 叙事顺序：品牌主张 → 四个特点 → 如何获取（四步，链向控制台）→ 上链锚定 → 使用入口 → 最终 CTA。
// 技术细节（TRIP / CBOR / 临界性）放在 /protocol 与 /architecture，首页只讲用户利益。
import { A } from "@solidjs/router";
import { dict, t } from "../../i18n";

const Pill = (props: { children: string }) => (
  <span class="inline-block text-[11px] rounded-full border border-white/15 bg-white/5 px-2.5 py-0.5 text-slate-300">
    {props.children}
  </span>
);

export default function Home() {
  const d = dict;
  return (
    <div>
      {/* Hero：品牌 + 主张 */}
      <section class="px-4 sm:px-8 pt-20 pb-24 text-center relative overflow-hidden">
        <div
          class="absolute inset-0 opacity-30 pointer-events-none"
          style={{
            background:
              "radial-gradient(600px 300px at 50% 0%, rgba(16,185,129,0.25), transparent)",
          }}
        />
        <div class="relative max-w-3xl mx-auto">
          <p class="text-xs font-mono text-emerald-400 mb-6">
            {d().home.hero.eyebrow}
          </p>
          <h1 class="text-5xl sm:text-7xl font-black tracking-tight leading-none bg-gradient-to-r from-emerald-300 via-emerald-400 to-cyan-400 bg-clip-text text-transparent">
            {d().home.hero.brand}
          </h1>
          <p class="mt-3 text-sm font-mono text-slate-400">
            {d().home.hero.chip}
          </p>
          <p class="mt-7 text-2xl sm:text-3xl font-bold tracking-tight text-slate-100">
            {d().home.hero.tagline}
          </p>
          <p class="mt-5 text-base sm:text-lg text-slate-300 leading-relaxed max-w-2xl mx-auto">
            {d().home.hero.sub}
          </p>
          <div class="mt-9 flex items-center justify-center gap-3 flex-wrap">
            <A
              href="/console/identity"
              class="rounded-lg bg-emerald-400 hover:bg-emerald-300 text-slate-950 font-bold px-7 py-3 transition-colors"
            >
              {t("home.hero.ctaGet")}
            </A>
            <a
              href="#how"
              class="rounded-lg border border-white/20 hover:bg-white/10 font-semibold px-7 py-3 transition-colors"
            >
              {t("home.hero.ctaHow")}
            </a>
          </div>
          <div class="mt-9 flex items-center justify-center gap-2 flex-wrap">
            {d().home.hero.badges.map((b) => (
              <Pill>{b}</Pill>
            ))}
          </div>
        </div>
      </section>

      {/* 四个特点 */}
      <section class="border-t border-white/10 bg-slate-900/50 px-4 sm:px-8 py-16">
        <div class="max-w-6xl mx-auto">
          <h2 class="text-2xl sm:text-3xl font-bold mb-10">
            {t("home.features.title")}
          </h2>
          <div class="grid gap-5 sm:grid-cols-2">
            {d().home.features.cards.map((f) => (
              <div class="rounded-xl border border-white/10 bg-white/[0.03] p-6">
                <h3 class="font-bold text-lg mb-2 text-emerald-300">{f.title}</h3>
                <p class="text-sm text-slate-400 leading-relaxed">{f.body}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* 如何获取 Geoyuan ID（四步） */}
      <section id="how" class="border-t border-white/10 px-4 sm:px-8 py-16 scroll-mt-16">
        <div class="max-w-6xl mx-auto">
          <p class="text-sm font-semibold text-emerald-400 mb-2">
            {t("home.obtain.eyebrow")}
          </p>
          <h2 class="text-2xl sm:text-3xl font-bold mb-10">
            {t("home.obtain.title")}
          </h2>
          <ol class="space-y-0">
            {d().home.obtain.items.map((s) => (
              <li class="flex gap-5 sm:gap-8 relative pb-10 last:pb-0">
                <div
                  class="absolute left-[22px] top-12 bottom-0 w-px bg-white/10 last:hidden"
                  aria-hidden="true"
                />
                <span class="relative z-10 shrink-0 h-11 w-11 rounded-full border border-emerald-400/40 bg-slate-950 text-emerald-400 font-mono font-bold flex items-center justify-center text-sm">
                  {s.n}
                </span>
                <div class="pt-1.5">
                  <h3 class="font-bold text-lg">{s.title}</h3>
                  <p class="text-sm text-slate-400 leading-relaxed mt-1 max-w-2xl">
                    {s.body}
                  </p>
                  <A
                    href={s.href}
                    class="inline-block mt-2.5 text-sm font-semibold text-emerald-400 hover:text-emerald-300"
                  >
                    {s.cta} →
                  </A>
                </div>
              </li>
            ))}
          </ol>
          <p class="mt-8 text-xs text-slate-400 max-w-2xl">
            {d().home.obtain.note}
          </p>
          <A
            href="/console/identity"
            class="inline-block mt-5 rounded-lg bg-emerald-400 hover:bg-emerald-300 text-slate-950 font-bold px-7 py-3 transition-colors"
          >
            {t("home.obtain.cta")}
          </A>
        </div>
      </section>

      {/* 上链锚定 */}
      <section class="border-t border-white/10 bg-slate-900/50 px-4 sm:px-8 py-16">
        <div class="max-w-6xl mx-auto">
          <p class="text-sm font-semibold text-emerald-400 mb-2">
            {t("home.onchain.eyebrow")}
          </p>
          <h2 class="text-2xl sm:text-3xl font-bold mb-4">
            {t("home.onchain.title")}
          </h2>
          <p class="text-sm text-slate-400 mb-10 max-w-2xl">
            {d().home.onchain.desc}
          </p>
          <div class="grid gap-5 md:grid-cols-3">
            {d().home.onchain.cards.map((c) => (
              <div class="rounded-xl border border-white/10 bg-white/[0.03] p-6">
                <h3 class="font-bold text-lg mb-2 text-slate-100">{c.title}</h3>
                <p class="text-sm text-slate-400 leading-relaxed">{c.body}</p>
              </div>
            ))}
          </div>
          <ul class="mt-8 flex flex-wrap gap-x-8 gap-y-2">
            {d().home.onchain.points.map((p) => (
              <li class="text-sm text-slate-300 flex items-center gap-2">
                <span class="text-emerald-400">✓</span>
                {p}
              </li>
            ))}
          </ul>
          <div class="mt-8 flex flex-wrap items-center gap-4">
            <span class="text-[11px] font-mono rounded-full border border-amber-400/30 text-amber-300 bg-amber-400/5 px-3 py-1">
              {d().home.onchain.network}
            </span>
            <A
              href="/architecture"
              class="text-sm font-semibold text-emerald-400 hover:text-emerald-300"
            >
              {t("home.onchain.cta")} →
            </A>
          </div>
        </div>
      </section>

      {/* 使用入口 */}
      <section class="border-t border-white/10 px-4 sm:px-8 py-16">
        <div class="max-w-6xl mx-auto">
          <h2 class="text-2xl sm:text-3xl font-bold mb-10">
            {t("home.platforms.title")}
          </h2>
          <div class="grid gap-5 md:grid-cols-3">
            {d().home.platforms.cards.map((f) => (
              <div class="rounded-xl border border-white/10 bg-white/[0.03] p-6">
                <div class="flex items-center justify-between mb-2">
                  <h3 class="font-bold text-lg">{f.name}</h3>
                  <span class="text-[11px] rounded-full border border-emerald-400/30 text-emerald-300 px-2 py-0.5">
                    {f.status}
                  </span>
                </div>
                <p class="text-sm text-slate-400 leading-relaxed">{f.desc}</p>
              </div>
            ))}
          </div>
          <div class="mt-8">
            <A
              href="/architecture"
              class="text-sm font-semibold text-emerald-400 hover:text-emerald-300"
            >
              {t("home.platforms.link")}
            </A>
          </div>
        </div>
      </section>

      {/* 最终 CTA */}
      <section class="border-t border-white/10 px-4 sm:px-8 py-20">
        <div class="max-w-3xl mx-auto text-center">
          <h2 class="text-2xl sm:text-3xl font-bold mb-4">
            {t("home.final.title")}
          </h2>
          <p class="text-slate-400 text-sm mb-8">{d().home.final.sub}</p>
          <A
            href="/console/identity"
            class="inline-block rounded-lg bg-emerald-400 hover:bg-emerald-300 text-slate-950 font-bold px-8 py-3 transition-colors"
          >
            {t("home.final.cta")}
          </A>
        </div>
      </section>
    </div>
  );
}
