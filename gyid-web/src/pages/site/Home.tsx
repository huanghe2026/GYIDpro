// 首页：TRIP 协议门户落地页。
// 叙事完全围绕 TRIP（轨迹即身份证明），旧 GyID 硬件指纹方案已弃用。
import { A } from "@solidjs/router";

const Badge = (props: { children: string }) => (
  <span class="inline-block text-[11px] font-mono rounded-full border border-white/15 bg-white/5 px-2.5 py-0.5 text-slate-300">
    {props.children}
  </span>
);

const problems = [
  {
    title: "证件 / 人脸 / 密码",
    body: "单点快照式核验：数据库一旦泄露即全面失守；人脸可被深度伪造，密码可被钓鱼与撞库。",
  },
  {
    title: "重放与代持攻击",
    body: "静态凭证可以录制后重放，也可以转手他人。一次性验证无法回答「现在操作账号的还是不是同一个人」。",
  },
  {
    title: "Sybil 女巫攻击",
    body: "批量注册机器人账号成本极低，DAO 投票、空投、社区治理因此被刷票与套利者淹没。",
  },
];

const steps = [
  {
    n: "01",
    title: "设备本地采集",
    body: "手机 GNSS、WiFi 指纹与 IMU 传感器持续采样。原始坐标只留在设备上，使用 H3 六边形网格做有损量化，不可逆。",
  },
  {
    n: "02",
    title: "签名面包屑链",
    body: "每个时间窗口生成一条 breadcrumb：H3 cell、时间戳、探索模式与环境证据用 Ed25519 设备密钥签名，前一条哈希入链（RFC 8949 确定性 CBOR）。",
  },
  {
    n: "03",
    title: "主动验证",
    body: "依赖方提供随机 nonce，Verifier 经 WebSocket 实时下发挑战；只有在轨迹现场的设备才能即时签名应答（draft-03 起已彻底删除被动模式）。",
  },
  {
    n: "04",
    title: "临界性评估",
    body: "Verifier 临界引擎计算莱维飞行指数 β、1/f 粉红噪声 PSD 缩放指数 α 与临界性置信度，区分人类移动与机器合成轨迹。",
  },
  {
    n: "05",
    title: "PoH 证书与 TIT",
    body: "通过则签发 Verifier 签名的人类存在证明（PoH），输出轨迹身份令牌 TIT——只含统计参数，不含任何地理位置。",
  },
];

const features = [
  {
    title: "原始 GPS 永不出设备",
    body: "H3 量化在本地完成，链上只有 cell 编号与统计特征；PoH 证书不含位置信息，Verifier 也无法还原你的轨迹。",
  },
  {
    title: "统计物理抗伪造",
    body: "人类位移服从截断莱维飞行并具有独特的 1/f PSD 特征（Parisi、Barabási 研究），脚本伪造轨迹难以复现。",
  },
  {
    title: "nonce 绑定防重放",
    body: "每份证明绑定依赖方一次性 nonce 与短有效期，证明不可录制、不可转借、不可跨站复用。",
  },
  {
    title: "伪匿名身份",
    body: "身份是自生成的 Ed25519 密钥（DID），不绑定姓名与证件；支持多独立验证方，信任可以分散。",
  },
];

const frontends = [
  {
    name: "Web（WASM）",
    desc: "SolidJS + wasm-bindgen：浏览器内生成密钥、采集与签名，本页即其门户。",
    status: "可用",
  },
  {
    name: "命令行 gyid",
    desc: "Rust 单二进制：identity / collect / verify / poh / explorer，开发者与自动化首选。",
    status: "可用",
  },
  {
    name: "Android",
    desc: "Kotlin + Compose + UniFFI：前台服务采集 FusedLocation / WiFi / IMU，断点续传。",
    status: "源码就绪",
  },
];

export default function Home() {
  return (
    <div>
      {/* Hero */}
      <section class="px-4 sm:px-8 pt-20 pb-24 text-center relative overflow-hidden">
        <div
          class="absolute inset-0 opacity-30 pointer-events-none"
          style={{
            background:
              "radial-gradient(600px 300px at 50% 0%, rgba(16,185,129,0.25), transparent)",
          }}
        />
        <div class="relative max-w-3xl mx-auto">
          <p class="text-xs font-mono text-emerald-400 mb-5">
            IETF Internet-Draft · draft-ayerbe-trip-protocol-04 · RATS WG
          </p>
          <h1 class="text-4xl sm:text-6xl font-black tracking-tight leading-tight">
            用真实的移动轨迹，
            <br />
            证明你是<span class="bg-gradient-to-r from-emerald-400 to-cyan-400 bg-clip-text text-transparent">真人</span>
          </h1>
          <p class="mt-6 text-lg text-slate-300 leading-relaxed">
            TRIP 不依赖证件、人脸或密码。它把人在物理世界持续的移动行为编码成
            加密签名的面包屑链，由验证方通过统计物理模型判定——
            <span class="text-slate-100">机器人无法伪造一条真实生活的轨迹</span>。
          </p>
          <div class="mt-9 flex items-center justify-center gap-3 flex-wrap">
            <A
              href="/console"
              class="rounded-lg bg-emerald-400 hover:bg-emerald-300 text-slate-950 font-bold px-6 py-3 transition-colors"
            >
              进入控制台
            </A>
            <A
              href="/protocol"
              class="rounded-lg border border-white/20 hover:bg-white/10 font-semibold px-6 py-3 transition-colors"
            >
              了解 TRIP 协议
            </A>
          </div>
          <div class="mt-8 flex items-center justify-center gap-2 flex-wrap">
            <Badge>Ed25519 签名</Badge>
            <Badge>H3 空间量化</Badge>
            <Badge>确定性 CBOR</Badge>
            <Badge>WebSocket 主动验证</Badge>
            <Badge>PoH / TIT</Badge>
          </div>
        </div>
      </section>

      {/* 问题 */}
      <section class="border-t border-white/10 bg-slate-900/50 px-4 sm:px-8 py-16">
        <div class="max-w-6xl mx-auto">
          <p class="text-sm font-semibold text-emerald-400 mb-2">为什么需要新方案</p>
          <h2 class="text-2xl sm:text-3xl font-bold mb-10">传统线上身份验证的三道裂缝</h2>
          <div class="grid gap-5 md:grid-cols-3">
            {problems.map((p) => (
              <div class="rounded-xl border border-white/10 bg-white/[0.03] p-6">
                <h3 class="font-bold text-lg mb-2 text-slate-100">{p.title}</h3>
                <p class="text-sm text-slate-400 leading-relaxed">{p.body}</p>
              </div>
            ))}
          </div>
          <p class="mt-8 text-sm text-slate-400">
            TRIP 的回答：<span class="text-slate-200">证明不是一次性快照，而是跨时间的持续物理存在</span>。
          </p>
        </div>
      </section>

      {/* 工作流 */}
      <section class="border-t border-white/10 px-4 sm:px-8 py-16">
        <div class="max-w-6xl mx-auto">
          <p class="text-sm font-semibold text-emerald-400 mb-2">工作原理</p>
          <h2 class="text-2xl sm:text-3xl font-bold mb-10">从传感器到人类存在证明</h2>
          <ol class="space-y-0">
            {steps.map((s) => (
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
                  <p class="text-sm text-slate-400 leading-relaxed mt-1 max-w-2xl">{s.body}</p>
                </div>
              </li>
            ))}
          </ol>
        </div>
      </section>

      {/* 特性 */}
      <section class="border-t border-white/10 bg-slate-900/50 px-4 sm:px-8 py-16">
        <div class="max-w-6xl mx-auto">
          <h2 class="text-2xl sm:text-3xl font-bold mb-10">四个设计支柱</h2>
          <div class="grid gap-5 sm:grid-cols-2">
            {features.map((f) => (
              <div class="rounded-xl border border-white/10 bg-white/[0.03] p-6">
                <h3 class="font-bold text-lg mb-2 text-emerald-300">{f.title}</h3>
                <p class="text-sm text-slate-400 leading-relaxed">{f.body}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* 三端 */}
      <section class="border-t border-white/10 px-4 sm:px-8 py-16">
        <div class="max-w-6xl mx-auto">
          <p class="text-sm font-semibold text-emerald-400 mb-2">GyID 开源实现</p>
          <h2 class="text-2xl sm:text-3xl font-bold mb-4">一套协议核心，三种前端</h2>
          <p class="text-sm text-slate-400 mb-10 max-w-2xl">
            协议逻辑集中在 Rust crate（trip-core / gyid-shared），三端共享同一份签名、
            链校验与 PoH 验证代码。Verifier（trip-server）提供 HTTP + WebSocket 接口。
          </p>
          <div class="grid gap-5 md:grid-cols-3">
            {frontends.map((f) => (
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
              查看系统架构与 Verifier API →
            </A>
          </div>
        </div>
      </section>

      {/* 状态条 + CTA */}
      <section class="border-t border-white/10 px-4 sm:px-8 py-16">
        <div class="max-w-4xl mx-auto text-center">
          <div class="flex items-center justify-center gap-8 sm:gap-14 flex-wrap mb-12">
            {[
              ["draft-04", "协议版本"],
              ["137", "workspace 测试"],
              ["10", "Verifier 端点"],
              ["3", "前端"],
            ].map(([num, label]) => (
              <div>
                <p class="text-3xl font-black text-emerald-400 font-mono">{num}</p>
                <p class="text-xs text-slate-400 mt-1">{label}</p>
              </div>
            ))}
          </div>
          <h2 class="text-2xl sm:text-3xl font-bold mb-4">亲自跑一条轨迹链</h2>
          <p class="text-slate-400 text-sm mb-8">
            在浏览器内生成身份、采集面包屑、发起主动验证并校验 PoH——全程无需后端账户。
          </p>
          <A
            href="/console"
            class="inline-block rounded-lg bg-emerald-400 hover:bg-emerald-300 text-slate-950 font-bold px-8 py-3 transition-colors"
          >
            打开控制台
          </A>
        </div>
      </section>
    </div>
  );
}
