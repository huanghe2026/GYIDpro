// 协议科普页：TRIP draft-04 是什么。内容整理自 src/public/trip.md。
const timeline = [
  {
    v: "-01",
    date: "2026 年初",
    title: "首次提交",
    body: "引入 Criticality Engine（临界引擎），将 Parisi 与 Barabási 的统计物理模型作为核心判别器，区分生物移动与合成轨迹。",
  },
  {
    v: "-02",
    date: "2026-02-09",
    title: "映射 RATS + 主动验证",
    body: "正式映射到 RATS 架构（RFC 9334）；新增主动验证协议（Active Verification），修复隐私模型；此版仍保留被动验证。",
  },
  {
    v: "-03",
    date: "2026-05-07",
    title: "统计物理评审后的大修订",
    body: "「Parisi 因子」改为标准谱分析术语 PSD 缩放指数 α；补齐 Lévy β 与 α 的数学推导；给出最小采样点收敛分析；彻底删除被动模式，所有证明绑定 RP nonce，杜绝重放。",
  },
  {
    v: "-04",
    date: "2026-05-08",
    title: "当前实现版本",
    body: "附录横向对比 EAT 位置声明、Proximate Location、可验证地理围栏、PoP 等方案；编辑勘误。文档有效期至 2026-11-09。GyID 当前即按本版实现。",
  },
];

const roles = [
  {
    en: "Attester",
    cn: "证明方（手机）",
    body: "本地采集 GNSS/IMU、H3 量化、签名面包屑链，并在主动验证中对实时挑战做签名应答。",
  },
  {
    en: "Verifier",
    cn: "验证方（临界引擎）",
    body: "校验面包屑签名与哈希链，运行统计物理评估（α / β / 置信度），签发 PoH 证书。GyID 中由 trip-server 实现。",
  },
  {
    en: "Relying Party",
    cn: "依赖方（业务服务）",
    body: "提供一次性 nonce 发起验证，消费 PoH/TIT 做出业务决策，例如投票防女巫、远程考勤。",
  },
];

const glossary = [
  ["Breadcrumb", "面包屑：一个时间窗口的轨迹证据片段，含 H3 cell、时间戳、环境证据与设备签名，前一片段哈希入链。"],
  ["H3", "Uber 开源的六边形地球网格，分辨率 0–15。TRIP 用它在设备本地对经纬度做不可逆有损量化（draft §2.1 MUST）。"],
  ["PoH", "Proof-of-Humanity，人类存在证明证书：Verifier 签名、绑定 RP nonce、短有效期，只携带统计参数。"],
  ["TIT", "Trajectory Identity Token，轨迹身份令牌：基于长期轨迹的伪匿名身份标识，不绑定现实姓名。"],
  ["DID", "TRIP 身份：设备自生成的 Ed25519 密钥对，公钥即标识，无需注册机构。"],
  ["α / β", "PSD 缩放指数 α 与截断莱维飞行指数 β：刻画 1/f 粉红噪声与人类位移分布的统计量，临界引擎的核心判据。"],
  ["Active Verification", "主动验证：RP nonce + Verifier 实时挑战 + 即时应答，证明「此刻在轨迹现场」。draft-03 起为唯一模式。"],
];

const usecases = [
  "DAO / 链上投票 / 空投的防女巫（Proof-of-Humanity）",
  "远程考试、劳务平台的跨时间持续活体核验",
  "金融与政务账号的身份连续性第二因子",
  "隐私优先的地理可信服务（区域权益、访问权限）",
  "物联网 / 车联网中人类操作者与自动设备的行为区分",
];

const limits = [
  "机器人/无人机搭载手机模拟 GPS+IMU 是最难防御的攻击，仍属开放研究问题；",
  "需要足够样本：至少约 200 条面包屑才能得到高置信度判定，短轨迹易误判；",
  "对行动不便、长期静止用户内置适配方案，以时序连续性替代空间移动要求；",
  "不内置人名绑定，输出的是伪匿名 TIT，不直接关联现实身份。",
];

function H2(props: { eyebrow: string; title: string }) {
  return (
    <div class="mb-8">
      <p class="text-xs font-mono text-emerald-400 mb-1">{props.eyebrow}</p>
      <h2 class="text-2xl sm:text-3xl font-bold">{props.title}</h2>
    </div>
  );
}

export default function Protocol() {
  return (
    <div class="px-4 sm:px-8 py-14">
      <div class="max-w-4xl mx-auto">
        {/* 头部 */}
        <p class="text-xs font-mono text-emerald-400 mb-3">
          IETF RATS WG · Internet-Draft（独立提交）
        </p>
        <h1 class="text-3xl sm:text-5xl font-black tracking-tight">
          TRIP：基于轨迹的身份证明
        </h1>
        <p class="mt-5 text-slate-300 leading-relaxed">
          Trajectory-based Recognition of Identity Proof——
          由 Camilo Ayerbe Posada（ULISSY s.r.l.）与 M. Usama Sardar（TU Dresden）
          撰写的密码学 + 人类移动行为身份认证协议。GyID 是该草案
          <span class="text-white"> draft-04 版本的开源工程实现</span>。
        </p>
        <div class="mt-6 rounded-lg border border-amber-400/30 bg-amber-400/10 px-4 py-3 text-sm text-amber-200">
          注意：TRIP 是正在迭代的互联网草案，<b>尚未成为正式 RFC</b>，
          随时可能修改或废弃，无大规模生产部署；本项目仅供研究与实验。
        </div>

        {/* 背景 */}
        <section class="mt-16">
          <H2 eyebrow="BACKGROUND" title="核心思路：轨迹不可伪造" />
          <div class="space-y-4 text-sm text-slate-300 leading-relaxed">
            <p>
              传统线上验证依赖证件、人脸、密码，存在单点泄露、深度伪造、重放与 Sybil
              女巫攻击。TRIP 提出完全不同的路径：
              <span class="text-white">把人在现实世界持续的物理移动轨迹，作为「真人存在」的证明。</span>
            </p>
            <p>其理论根基来自两项研究：</p>
            <ul class="list-disc pl-5 space-y-2 text-slate-400">
              <li>
                Giorgio Parisi（诺贝尔物理学奖）关于生物系统无标度关联与自组织临界（SOC）的研究；
              </li>
              <li>
                Albert-László Barabási 团队对人类移动模式的研究：人类位移服从
                <span class="text-slate-200">截断莱维飞行（Truncated Lévy flights）</span>，
                具有独特的 1/f 粉红噪声功率谱特征，脚本合成轨迹很难复现。
              </li>
            </ul>
          </div>
        </section>

        {/* 时间线 */}
        <section class="mt-16">
          <H2 eyebrow="TIMELINE" title="草案版本演进" />
          <div class="space-y-4">
            {timeline.map((t) => (
              <div
                class={`rounded-xl border p-5 ${
                  t.v === "-04"
                    ? "border-emerald-400/40 bg-emerald-400/5"
                    : "border-white/10 bg-white/[0.03]"
                }`}
              >
                <div class="flex items-baseline gap-3 flex-wrap">
                  <span class="font-mono font-black text-lg text-emerald-400">{t.v}</span>
                  <span class="text-xs text-slate-500 font-mono">{t.date}</span>
                  <span class="font-bold">{t.title}</span>
                  {t.v === "-04" && (
                    <span class="text-[11px] rounded-full bg-emerald-400/20 text-emerald-300 px-2 py-0.5">
                      GyID 实现版本
                    </span>
                  )}
                </div>
                <p class="mt-2 text-sm text-slate-400 leading-relaxed">{t.body}</p>
              </div>
            ))}
          </div>
        </section>

        {/* RATS 角色 */}
        <section class="mt-16">
          <H2 eyebrow="RATS ROLES" title="三方角色（RFC 9334 模型）" />
          <div class="grid gap-4 sm:grid-cols-3">
            {roles.map((r) => (
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
          <H2 eyebrow="GLOSSARY" title="核心概念速查" />
          <dl class="rounded-xl border border-white/10 overflow-hidden">
            {glossary.map(([term, desc], i) => (
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
          <H2 eyebrow="PRIVACY" title="隐私设计要点" />
          <ul class="list-disc pl-5 space-y-2 text-sm text-slate-400 leading-relaxed">
            <li>原始 GPS 坐标在设备本地用 H3 网格量化，不可逆有损压缩，<span class="text-slate-200">原始坐标不外传</span>；</li>
            <li>对外交付的 PoH 证书只含统计参数（α、β、可信度分数），<span class="text-slate-200">不含任何地理位置</span>；</li>
            <li>支持多个独立验证方，依赖方可自行选择信任节点；</li>
            <li>协议传输无关，不绑定特定区块链或域名系统，可独立运行。</li>
          </ul>
        </section>

        {/* 局限 */}
        <section class="mt-16">
          <H2 eyebrow="LIMITATIONS" title="已知局限" />
          <ul class="list-disc pl-5 space-y-2 text-sm text-slate-400 leading-relaxed">
            {limits.map((l) => (
              <li>{l}</li>
            ))}
          </ul>
        </section>

        {/* 场景 */}
        <section class="mt-16">
          <H2 eyebrow="USE CASES" title="预期应用场景" />
          <div class="grid gap-3 sm:grid-cols-2">
            {usecases.map((u) => (
              <div class="rounded-lg border border-white/10 bg-white/[0.03] px-4 py-3 text-sm text-slate-300">
                {u}
              </div>
            ))}
          </div>
          <p class="mt-5 text-xs text-slate-500 leading-relaxed">
            TRIP <span class="text-slate-300">不是一次性位置证明</span>，
            核心价值是时序连续的移动行为统计证明；与 EAT、PoP 等单点位置证明协议互为补充而非替代。
            上述场景多为草案设想，公开大规模落地案例极少。
          </p>
        </section>
      </div>
    </div>
  );
}
