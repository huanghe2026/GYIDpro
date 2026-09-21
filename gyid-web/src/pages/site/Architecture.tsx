// 架构页：GyID 的 crate 地图、数据流、Verifier API 与开发者快速开始。
import { A } from "@solidjs/router";

const crates = [
  {
    name: "trip-core",
    lang: "Rust",
    role: "协议核心",
    body: "draft-04 原语：确定性 CBOR 编解码、breadcrumb 签名/校验、哈希链、PoH 证书签发与验证、DID、H3 量化辅助。无平台依赖，70+ 单测。",
  },
  {
    name: "gyid-shared",
    lang: "Rust",
    role: "共享业务层",
    body: "三端共用：身份生成与加密存储（PBKDF2+AES-GCM）、采集封装、链管理、Verifier 客户端、PoH 策略校验。feature gate 区分 wasm / native。",
  },
  {
    name: "trip-server",
    lang: "Rust / Axum",
    role: "Verifier 服务",
    body: "RATS 验证方：证据合并校验、主动验证挑战（WebSocket）、临界引擎评估、PoH 签发、DID/TIT、Explorer 聚合；内存存储，CORS 可配。",
  },
  {
    name: "gyid-wasm",
    lang: "Rust → WASM",
    role: "浏览器桥",
    body: "wasm-bindgen + serde-wasm-bindgen + tsify，把共享核心编译为 wasm32 并自动生成 TypeScript 类型。",
  },
  {
    name: "gyid-web",
    lang: "SolidJS + Vite",
    role: "Web 前端",
    body: "本站点：TRIP 门户 + 身份/采集/导入/验证/证书/Explorer 控制台，浏览器内完成全部密码学操作。",
  },
  {
    name: "trip-cli（gyid）",
    lang: "Rust",
    role: "命令行",
    body: "二进制名 gyid：identity / collect / verify / poh / explorer 用户命令，外加 chain / cbor / key 等协议调试命令。",
  },
  {
    name: "gyid-android-rs + gyid-android",
    lang: "UniFFI + Compose",
    role: "Android",
    body: "Rust 经 UniFFI 生成 Kotlin 绑定；前台服务采集 FusedLocation / WiFi / IMU，300 条分批断点续传，8 步验证状态机。",
  },
  {
    name: "contracts/GeoTITRegistry",
    lang: "Solidity",
    role: "链上锚定",
    body: "TIT 注册合约（Foundry）：把轨迹身份锚定到链上，绑定公钥与长期链摘要，供链上应用做防女巫查询。",
  },
];

const endpoints = [
  ["POST", "/v1/evidence", "上传面包屑 CBOR 帧，服务端校验签名/链规则后按链合并"],
  ["GET", "/v1/identity/:hex", "查询 attester 的面包屑数、独立 cell、链头与最后时间戳"],
  ["POST", "/v1/verify", "RP 发起主动验证：提交 attester + nonce，创建挑战"],
  ["GET", "/v1/challenge", "WebSocket 挑战通道：ready 握手 → 下发 challenge → 收签名应答"],
  ["POST", "/v1/poh", "凭 challenge_id 取 PoH（200 CBOR / 202 pending / 410 过期）"],
  ["GET", "/v1/pohs?attester=", "列出某 attester 的全部 PoH challenge"],
  ["GET", "/v1/explorer", "全网身份总览：全部 attester 统计 + PoH 计数，按活跃度排序"],
  ["GET", "/v1/did/:did", "解析 TRIP DID 为公钥与身份信息"],
  ["GET", "/v1/tit/:hex", "按 attester 公钥计算/签发轨迹身份令牌 TIT"],
  ["GET", "/.well-known/verifier.json", "Verifier 元数据与公钥（客户端信任锚点）"],
];

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
  return (
    <div class="px-4 sm:px-8 py-14">
      <div class="max-w-5xl mx-auto">
        <p class="text-xs font-mono text-emerald-400 mb-3">ENGINEERING</p>
        <h1 class="text-3xl sm:text-5xl font-black tracking-tight">系统架构</h1>
        <p class="mt-5 text-slate-300 leading-relaxed max-w-3xl">
          协议逻辑只写一次（Rust），经编译/绑定分发到三端；Verifier 以无状态友好的
          HTTP + WebSocket 暴露服务。传输与区块链无关，链上锚定为可选层。
        </p>

        {/* 数据流 */}
        <section class="mt-14">
          <H2 eyebrow="DATA FLOW" title="一次证明的生命周期" />
          <div class="rounded-xl border border-white/10 bg-white/[0.03] p-6">
            <ol class="space-y-3 text-sm text-slate-300">
              {[
                "Attester 端生成 Ed25519 身份（DID），持续采集并签名面包屑，本地维护哈希链；",
                "面包屑以确定性 CBOR 帧分批 POST 到 Verifier，服务端执行间隔/重复 cell/探索模式等链规则校验；",
                "RP 用自己的随机 nonce 调 POST /v1/verify；Attester 经 WS 接收挑战并在链头现场签名应答；",
                "Verifier 复算统计物理指标（α、β、置信度）并检查链完整性，通过则签发 PoH；",
                "RP 轮询取得 PoH CBOR，用 Verifier 公钥与自己的 nonce 在本地校验新鲜度与策略门槛；",
                "可选：将 TIT 与链摘要写入 GeoTITRegistry 合约，供链上应用防女巫查询。",
              ].map((t, i) => (
                <li class="flex gap-3">
                  <span class="font-mono text-emerald-400 shrink-0">{i + 1}.</span>
                  <span class="text-slate-400 leading-relaxed">{t}</span>
                </li>
              ))}
            </ol>
          </div>
        </section>

        {/* Crate 地图 */}
        <section class="mt-16">
          <H2 eyebrow="MODULES" title="组件地图" />
          <div class="grid gap-4 sm:grid-cols-2">
            {crates.map((c) => (
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
            eyebrow="VERIFIER API"
            title="HTTP / WebSocket 端点"
            desc="默认监听 :8080，TRIP_CORS_ORIGINS 环境变量配置跨域（逗号分隔，* 为通配）。"
          />
          <div class="rounded-xl border border-white/10 overflow-hidden">
            {endpoints.map(([m, path, desc], i) => (
              <div
                class={`grid sm:grid-cols-[70px_230px_1fr] gap-2 sm:gap-4 px-4 py-3 text-sm items-start ${
                  i % 2 === 0 ? "bg-white/[0.03]" : "bg-transparent"
                }`}
              >
                <span
                  class={`justify-self-start text-[11px] font-mono font-bold rounded border px-1.5 py-0.5 ${methodColor[m]}`}
                >
                  {m}
                </span>
                <code class="text-xs text-slate-200 break-all">{path}</code>
                <span class="text-xs text-slate-400 leading-relaxed">{desc}</span>
              </div>
            ))}
          </div>
        </section>

        {/* 开发者快速开始 */}
        <section class="mt-16">
          <H2 eyebrow="QUICK START" title="开发者快速开始" />
          <div class="space-y-6">
            <div>
              <p class="text-sm font-semibold mb-2 text-slate-200">1. 跑测试（workspace 全绿）</p>
              <Code>{`git clone git@github.com:huanghe2026/GYIDpro.git
cd GYIDpro
cargo test --workspace`}</Code>
            </div>
            <div>
              <p class="text-sm font-semibold mb-2 text-slate-200">2. 启动 Verifier</p>
              <Code>{`cargo run -p trip-server --release
# → trip-server listening on 0.0.0.0:8080`}</Code>
            </div>
            <div>
              <p class="text-sm font-semibold mb-2 text-slate-200">3. CLI 走一遍身份与采集</p>
              <Code>{`gyid identity new --label me
gyid collect --res 7 --interval 900     # 采集并签名面包屑
gyid verify                              # RP 主动验证
gyid explorer                            # 全网身份总览`}</Code>
            </div>
            <div>
              <p class="text-sm font-semibold mb-2 text-slate-200">4. Web 前端（本页面所属）</p>
              <Code>{`cd gyid-wasm && wasm-pack build --target web --release
cd ../gyid-web && pnpm install && pnpm dev
# VITE_VERIFIER_URL 可指向远端 Verifier`}</Code>
            </div>
          </div>
          <div class="mt-8 rounded-lg border border-white/10 bg-white/[0.03] px-4 py-3 text-sm text-slate-400">
            想直接体验？
            <A href="/console" class="text-emerald-400 hover:text-emerald-300 font-semibold">
              打开 Web 控制台 →
            </A>
          </div>
        </section>
      </div>
    </div>
  );
}
