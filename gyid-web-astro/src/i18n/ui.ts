/**
 * 全站文案与路由工具（单一事实源）。
 *
 * 双语约定与现有 gyid.geoyuan.com 一致：**英文默认**（`/`），中文在 `/zh/` 下。
 * 页面通过 `path(lang, slug)` 生成链接，避免手写前缀导致中英互串。
 */

export type Lang = 'en' | 'zh';

export const LANGS: Lang[] = ['en', 'zh'];
export const DEFAULT_LANG: Lang = 'en';

/**
 * 部署基路径（来自 `astro.config.mjs` 的 `base`，由 `BASE_PATH` 环境变量决定）：
 * 根部署为空串 ``，子目录部署为 `/verify`。末尾斜杠已剥掉，便于拼接。
 */
const BASE = import.meta.env.BASE_URL.replace(/\/+$/, '');

/** slug → 路径。slug 为空串表示首页。 */
export function path(lang: Lang, slug = ''): string {
  const clean = slug.replace(/^\/+|\/+$/g, '');
  const seg = lang === 'zh' ? '/zh' : '';
  const full = `${BASE}${seg}/${clean}`.replace(/\/{2,}/g, '/').replace(/\/+$/, '');
  return full || '/';
}

/** 当前页在另一种语言下的对应路径。 */
export function altPath(lang: Lang, slug: string): string {
  return path(lang === 'en' ? 'zh' : 'en', slug);
}

export const SITE = {
  name: 'GyID',
  nameFull: 'Geoyuan ID',
  domain: 'gyid.geoyuan.com',
  origin: 'https://gyid.geoyuan.com',
  github: 'https://github.com/huanghe2026/GYIDpro',
  draft: 'draft-ayerbe-trip-protocol-04',
  icp: '浙ICP备2024115135号-1',
};

export interface NavItem {
  slug: string;
  label: string;
}

export interface Copy {
  htmlLang: string;
  meta: { siteTitle: string; description: string };
  nav: {
    home: string;
    protocol: string;
    architecture: string;
    docs: string;
    console: string;
    about: string;
    github: string;
    langSwitch: string;
  };
  console: {
    title: string;
    lead: string;
    tabs: { identity: string; collect: string; import: string; verify: string };
    verifier: string;
    unlocked: string;
    locked: string;
    lock: string;
    unlock: string;
    disclaimer: string;
  };
  footer: {
    tagline: string;
    product: string;
    resources: string;
    legal: string;
    draftNote: string;
    rights: string;
  };
  home: HomeCopy;
  protocol: ProtocolCopy;
  architecture: ArchitectureCopy;
  about: AboutCopy;
  docs: DocsCopy;
}

export interface HomeCopy {
  eyebrow: string;
  title: string;
  subtitle: string;
  lead: string;
  ctaPrimary: string;
  ctaSecondary: string;
  badges: string[];
  featuresTitle: string;
  featuresLead: string;
  features: { title: string; body: string; accent: 'emerald' | 'violet' | 'amber' | 'pink' }[];
  flowTitle: string;
  flowLead: string;
  steps: { n: string; title: string; body: string }[];
  engineTitle: string;
  engineLead: string;
  engine: { title: string; body: string }[];
  trustTitle: string;
  trustLead: string;
  /** [level, breadcrumb range, meaning] */
  trustRows: string[][];
  stackTitle: string;
  stackLead: string;
  stack: { title: string; body: string; tag: string }[];
}

export interface ProtocolCopy {
  eyebrow: string;
  title: string;
  lead: string;
  sections: { title: string; body: string; points: string[] }[];
  tableTitle: string;
  tableHead: string[];
  tableRows: string[][];
  note: string;
}

export interface ArchitectureCopy {
  eyebrow: string;
  title: string;
  lead: string;
  layersTitle: string;
  layers: { name: string; role: string; items: string[] }[];
  repoTitle: string;
  repo: { crate: string; role: string }[];
  endpointsTitle: string;
  endpoints: { method: string; path: string; desc: string }[];
}

export interface AboutCopy {
  eyebrow: string;
  title: string;
  lead: string;
  sections: { title: string; body: string; points?: string[] }[];
  statusTitle: string;
  statusRows: string[][];
}

export interface DocsCopy {
  indexTitle: string;
  indexLead: string;
  cards: { slug: string; title: string; desc: string }[];
  apiTitle: string;
  apiLead: string;
  backToDocs: string;
}

const en: Copy = {
  htmlLang: 'en',
  meta: {
    siteTitle: 'GyID — Proof of Humanity from real trajectories',
    description:
      'GyID is an open-source implementation of the IETF TRIP draft: quantized breadcrumbs, hash-chain evidence, Active Verification and Proof-of-Humanity certificates. Raw GPS never leaves the device.',
  },
  nav: {
    home: 'Home',
    protocol: 'Protocol',
    architecture: 'Architecture',
    docs: 'Docs',
    console: 'Console',
    about: 'About',
    github: 'GitHub',
    langSwitch: '中文',
  },
  console: {
    title: 'Console',
    lead: 'Everything runs in your browser. The identity key is generated locally and encrypted with your passphrase — it never leaves this device.',
    tabs: { identity: 'Identity', collect: 'Collect', import: 'Import', verify: 'Verify' },
    verifier: 'Verifier',
    unlocked: 'Unlocked (key in memory only)',
    locked: 'Locked',
    lock: 'Lock',
    unlock: 'Unlock',
    disclaimer:
      'Research prototype. The verifier on this domain is a reference implementation on a dev key — do not rely on it for production security decisions.',
  },
  footer: {
    tagline: 'Proof of humanity, without disclosing where you have been.',
    product: 'Product',
    resources: 'Resources',
    legal: 'Legal',
    draftNote: 'An open-source implementation of IETF Internet-Draft',
    rights: 'MIT OR Apache-2.0',
  },
  home: {
    eyebrow: 'Built on the IETF TRIP draft',
    title: 'Prove you are human with the road you actually walked',
    subtitle: 'Geoyuan ID — short for GyID',
    lead: 'GyID is a non-transferable, non-forgeable identity that never reveals your location. No documents, no face scan, no password. It accumulates from the way you really live and move.',
    ctaPrimary: 'Open the console',
    ctaSecondary: 'Read the protocol',
    badges: [
      'No documents, face or password',
      'Raw GPS never uploaded',
      'One identity per person',
      'Verifiable on-chain',
    ],
    featuresTitle: 'Why GyID is different',
    featuresLead:
      'Bots can register ten thousand accounts overnight, but they cannot fake a life. Human movement has a statistical signature that scripts do not reproduce.',
    features: [
      {
        title: 'Only a real person can hold one',
        body: 'Human displacement follows a power law: the spectrum of your movement decays like 1/f^α with α in the pink-noise band. Synthetic walkers land flat (α≈0) or drift brown (α≳1.2) — separable by a single statistic.',
        accent: 'emerald',
      },
      {
        title: 'Your location is never uploaded',
        body: 'Every fix is quantized on-device into an H3 cell (res 10, about 15,000 m²) and only the cell id, a context digest and a signature are transmitted. Even the verifier cannot reconstruct where you have been.',
        accent: 'violet',
      },
      {
        title: 'It cannot be bought, lent or held for you',
        body: 'Each presentation is bound to a fresh one-time challenge and answered live by the device holding the trajectory. Screenshots, recordings and account handovers all fail.',
        accent: 'amber',
      },
      {
        title: 'No name, no sign-up',
        body: 'Your identity is a key pair generated on your device. The relying party learns "a human being" — never who you are.',
        accent: 'pink',
      },
    ],
    flowTitle: 'Four steps to a Geoyuan ID',
    flowLead: 'Nothing is registered anywhere. Everything accumulates locally, then gets proven.',
    steps: [
      {
        n: '01',
        title: 'Create an identity',
        body: 'A key pair is generated in your browser and encrypted with a passphrase you choose. The seed only ever exists in memory while unlocked.',
      },
      {
        n: '02',
        title: 'Collect breadcrumbs',
        body: 'While you move, each fix becomes a signed breadcrumb: H3 cell, timestamp, context digest and previous block hash. One every 15 minutes or longer.',
      },
      {
        n: '03',
        title: 'Seal epochs and anchor',
        body: 'Every 100 breadcrumbs are sealed into a Merkle root. Roots are registered on-chain (Base) as a public existence proof — aggregate values only.',
      },
      {
        n: '04',
        title: 'Answer a live challenge',
        body: 'A relying party sends a nonce; the verifier pushes it over a real-time channel; your device signs it. The verifier issues a Proof-of-Humanity certificate bound to that nonce.',
      },
    ],
    engineTitle: 'The classic criticality engine',
    engineLead:
      'The protocol prescribes deterministic, interpretable statistics. GyID implements them byte-for-byte as the interoperability baseline; AI may only add separately-signed evidence.',
    engine: [
      {
        title: 'PSD scaling exponent α',
        body: 'Displacements between consecutive H3 cell centres form a series. A naive DFT gives the power spectrum; a log-log regression gives α. Biological range: 0.30–0.80, centred at 0.55.',
      },
      {
        title: 'Truncated Lévy fits β and κ',
        body: 'The step-length distribution P(Δr) ∝ Δr^(−β)·e^(−Δr/κ) is fitted per epoch by maximum likelihood. Human β typically falls in 1.50–1.90.',
      },
      {
        title: 'Levy–PSD bridge check',
        body: 'α ≈ (3 − β)·g with g expected in 0.3–0.7. A mismatch suggests data-quality problems or adversarial manipulation of a single statistic.',
      },
      {
        title: 'Six-component Hamiltonian',
        body: 'Spatial, temporal, kinetic, flock, contextual and structural terms combine into an anomaly score with NOMINAL / ELEVATED / SUSPICIOUS / CRITICAL levels.',
      },
    ],
    trustTitle: 'The trust score, and what each level means',
    trustLead:
      'T = 40·min(n/200,1) + 30·min(unique/50,1) + 20·min(days/365,1) + 10·chain_integrity. A failed criticality test caps T at 50.',
    trustRows: [
      ['Bootstrap', '0 – 63', 'PSD is not computed yet. Shown honestly as a growing identity.'],
      ['Provisional', '64 – 199', 'PSD available but high variance. Enough to claim a handle.'],
      ['Stable', '200 – 255', 'Variance below 0.05. Reliable positive classification.'],
      ['High confidence', '256+', 'Suitable for high-stakes relying-party decisions.'],
    ],
    stackTitle: 'What is in the repository',
    stackLead: 'Six Rust crates, one Solidity contract and two front-ends, all open source.',
    stack: [
      {
        title: 'trip-core',
        body: 'Deterministic CBOR, Ed25519, breadcrumb hash chain, epochs and Merkle roots, liveness, PoH certificates, DID/TIT, the criticality engine and EVM anchor encoding.',
        tag: 'Rust',
      },
      {
        title: 'trip-server',
        body: 'The verifier: evidence intake with chain rules, Active Verification over WebSocket, PoH issuance and the .well-known policy document.',
        tag: 'Rust',
      },
      {
        title: 'trip-cli',
        body: 'The gyid command: identity, collection, verification, did/tit, on-chain anchoring and population calibration.',
        tag: 'Rust',
      },
      {
        title: 'gyid-wasm + web console',
        body: 'The browser attester. All cryptography is the same Rust code compiled to WebAssembly — this site uses it directly.',
        tag: 'WASM',
      },
      {
        title: 'GeoTITRegistry.sol',
        body: 'On-chain existence registry: register a public key, anchor epoch roots, claim a handle. No tokens, no coordinates, verifier-gated.',
        tag: 'Solidity',
      },
      {
        title: 'gyid-android',
        body: 'Native collector with a foreground service, resumable upload and the same verification flow.',
        tag: 'Kotlin',
      },
    ],
  },
  protocol: {
    eyebrow: 'The protocol',
    title: 'What TRIP actually specifies',
    lead: 'GyID implements draft-ayerbe-trip-protocol-04 from the IETF RATS working group. The specification is deliberately narrow: it defines evidence, not transport, naming or anchoring. Those gaps are where this implementation contributes companion work.',
    sections: [
      {
        title: 'Breadcrumbs',
        body: 'The unit of evidence. A CBOR map with nine fields, signed with Ed25519; the block hash is SHA-256 over the full encoding.',
        points: [
          'index — position in the chain, contiguous from 0',
          'identity — Ed25519 public key of the attester',
          'timestamp — Unix seconds',
          'h3 cell + resolution — 7 to 10, default 10',
          'context digest — SHA-256 over h3 / time bucket / Wi-Fi / cell / IMU parts',
          'prev hash — previous block hash, null at genesis',
          'meta flags — including exploration sessions',
          'signature — covers the deterministic CBOR of fields 0–7',
        ],
      },
      {
        title: 'Chain rules',
        body: 'Rules that make fabrication expensive, applied identically on device and on the verifier.',
        points: [
          'Consecutive breadcrumbs in the same H3 cell are rejected',
          'At most 10 breadcrumbs accumulate per cell',
          'Minimum interval 15 minutes; explicit exploration sessions may shorten it to 5',
          'Indices must be contiguous and each breadcrumb must reference the previous block hash',
          'Every breadcrumb carries its own Ed25519 signature',
        ],
      },
      {
        title: 'Epochs and anchoring',
        body: 'Breadcrumbs are never written to a chain. They are sealed into a Merkle root, and only that root is anchored.',
        points: [
          'Default epoch size is 100 breadcrumbs',
          'The root commits to the ordered set, plus the count of unique cells',
          'The root is signed by the identity key',
          'Anchor targets are chain-agnostic; this implementation uses Base',
        ],
      },
      {
        title: 'Proof of Humanity certificate',
        body: 'The output a relying party actually consumes. Fifteen CBOR fields, signed by the verifier, containing statistics only.',
        points: [
          'identity, issued-at, epoch count, breadcrumb and unique-cell counts',
          'α, β, κ, π and the criticality confidence',
          'trust score and validity window',
          'RP nonce — mandatory, binds the certificate to one request',
          'chain head hash — mandatory, binds it to one exact evidence set',
        ],
      },
      {
        title: 'Active Verification',
        body: 'The only mode this deployment promotes. A pre-recorded trajectory cannot answer a challenge that did not exist when it was captured.',
        points: [
          'The relying party generates a 16-byte nonce',
          'The verifier pushes a challenge over a real-time channel',
          'The attester signs the nonce together with the chain head and expected index',
          'The verifier checks the response, then issues a PoH bound to that nonce',
        ],
      },
      {
        title: 'Multi-verifier by design',
        body: 'Any compliant implementation can act as a verifier with its own Ed25519 key; an attester may submit to several and a relying party picks whom to trust.',
        points: [
          'There is no privileged verifier in the protocol',
          'The verifier publishes its key and policy at a well-known URL',
          'This project positions itself as the first public verifier, not the only one',
        ],
      },
    ],
    tableTitle: 'Convergence regimes',
    tableHead: ['Breadcrumbs', 'Regime', 'What it supports'],
    tableRows: [
      ['0 – 63', 'Bootstrap', 'PSD is not defined; the identity is shown as still growing'],
      ['64 – 199', 'Provisional', 'PSD computed, variance around 0.15 — enough to claim a handle'],
      ['200 – 255', 'Stable', 'Variance below 0.05 — reliable positive classification'],
      ['256+', 'High confidence', 'Suitable for high-stakes decisions'],
    ],
    note: 'The specification is an Internet-Draft, not an RFC. It expires unless renewed, and this project pins draft-04 as its interoperability baseline while tracking later revisions.',
  },
  architecture: {
    eyebrow: 'Architecture',
    title: 'How the pieces fit together',
    lead: 'Three roles talk to each other: the attester holds the trajectory, the verifier computes the statistics, the relying party asks the question. Only aggregate values cross the boundary.',
    layersTitle: 'Layers',
    layers: [
      {
        name: 'Attester',
        role: 'Device side — holds the only copy of the trajectory',
        items: [
          'Generates and encrypts the identity key',
          'Quantizes GPS to H3 before anything leaves the device',
          'Signs each breadcrumb and maintains the local chain',
          'Answers live challenges and seals epochs',
        ],
      },
      {
        name: 'Verifier',
        role: 'Server side — computes statistics, never sees coordinates',
        items: [
          'Validates chain rules, signatures and ordering',
          'Runs the criticality engine over quantized cells',
          'Drives Active Verification over a WebSocket channel',
          'Issues PoH certificates under a published policy',
        ],
      },
      {
        name: 'Relying party',
        role: 'Whoever needs to know you are human',
        items: [
          'Generates a nonce and asks the verifier for a certificate',
          'Verifies the certificate signature against the published verifier key',
          'Reads only statistics — α, β, confidence, trust score',
        ],
      },
      {
        name: 'Anchor',
        role: 'Public existence proof, deliberately minimal',
        items: [
          'Registers the mapping from public key to DID suffix once',
          'Stores epoch Merkle roots with strictly contiguous numbering',
          'Stores handle claims above a threshold',
          'No tokens, no coordinates, no delegatecalls',
        ],
      },
    ],
    repoTitle: 'Workspace layout',
    repo: [
      { crate: 'trip-core', role: 'Protocol data structures, cryptography, criticality engine, anchor encoding' },
      { crate: 'trip-server', role: 'Verifier HTTP/WS service with in-memory storage' },
      { crate: 'trip-cli (gyid)', role: 'Command-line identity, collection, verification, anchoring, calibration' },
      { crate: 'gyid-shared', role: 'Business logic shared by CLI, web and mobile' },
      { crate: 'gyid-wasm', role: 'Browser attester bridge, compiled from the same Rust core' },
      { crate: 'gyid-web-astro', role: 'This site: portal, documentation and the online console' },
      { crate: 'contracts', role: 'GeoTITRegistry.sol and its compiler / in-process EVM tooling' },
      { crate: 'gyid-android', role: 'Native Android collector and verifier client' },
    ],
    endpointsTitle: 'Verifier API',
    endpoints: [
      { method: 'POST', path: '/v1/evidence', desc: 'Submit a batch of signed breadcrumbs; the server validates chain rules' },
      { method: 'GET', path: '/v1/identity/:hex', desc: 'Chain head, breadcrumb count and unique cell count for one attester' },
      { method: 'POST', path: '/v1/verify', desc: 'Create a challenge bound to an RP nonce (Active Verification)' },
      { method: 'WS', path: '/v1/challenge', desc: 'Real-time channel carrying challenges down and responses up' },
      { method: 'GET', path: '/v1/poh/:challenge_id', desc: 'Fetch the issued certificate for a challenge' },
      { method: 'GET', path: '/v1/explorer', desc: 'Aggregate view over all attesters known to this verifier' },
      { method: 'GET', path: '/.well-known/verifier.json', desc: 'Verifier public key, policy thresholds and published limits' },
    ],
  },
  about: {
    eyebrow: 'About',
    title: 'What this project is, and what it is not',
    lead: 'GyID — Geoyuan ID — is the first public open-source implementation of the IETF TRIP draft. It exists to make the specification concrete, testable and criticisable.',
    sections: [
      {
        title: 'A protocol implementation first',
        body: 'The value is in byte-exact conformance and published evidence, not in a product feature list. Golden test vectors, the Monte Carlo study and the population calibration are all in the repository so that other implementations can be checked against them.',
      },
      {
        title: 'Privacy by construction',
        body: 'The specification forbids raw GPS in any protocol message, and this implementation enforces it at the lowest level.',
        points: [
          'Coordinates are quantized to H3 on device before signing',
          'The chain stores aggregate values only — Merkle roots and counts',
          'No name, phone number or government identifier is involved',
          'The identity key is generated locally and encrypted with a user passphrase',
        ],
      },
      {
        title: 'Honest about open problems',
        body: 'The draft itself lists unresolved questions, and this project does not pretend to have closed them.',
        points: [
          'Single-trajectory ROC and confidence intervals need population calibration — see the W7 report',
          'The six Hamiltonian components are not statistically independent',
          'Robot / drone carried device attacks remain an active area of research',
          'Adaptive adversaries can try to shape a single statistic; the engine therefore never relies on one',
        ],
      },
      {
        title: 'What it is not',
        body: 'Not an IETF standard, not a product with a security guarantee, and not a way to identify who you are.',
        points: [
          'The draft is an Internet-Draft with no IETF endorsement',
          'The verifier on this domain runs on a development key',
          'No audit has been performed, and the contracts are testnet-only',
          'It answers "is this a human being", never "which human being"',
        ],
      },
    ],
    statusTitle: 'Implementation status',
    statusRows: [
      ['W1 – W6', 'Protocol core, chain rules, criticality engine, Hamiltonian, trust score, verifier service, WASM attester and demo pages', 'Complete'],
      ['W7', 'Population calibration against GeoLife plus a synthetic control baseline, real ROC / AUC, whitepaper draft', 'Complete'],
      ['W8', 'did:geoyuan and TIT, GeoTITRegistry on Base testnet, CLI anchoring commands', 'Complete'],
      ['W9 – W12', 'Neural evidence layer, adversarial arena, integration trial, mainnet and publication', 'Not started'],
    ],
  },
  docs: {
    indexTitle: 'Documentation',
    indexLead: 'Specification notes, the calibration report and the verifier API — written to be read alongside the draft.',
    cards: [
      {
        slug: 'docs/protocol-notes',
        title: 'Protocol implementation notes',
        desc: 'Field-by-field mapping between draft-04 and this implementation, including the gaps this project fills.',
      },
      {
        slug: 'docs/calibration',
        title: 'Population calibration report',
        desc: 'Alpha-boundary calibration, synthetic control families, ROC / AUC and what the numbers do and do not support.',
      },
      {
        slug: 'docs/api',
        title: 'Verifier API reference',
        desc: 'Every endpoint this deployment exposes, with request and response shapes and the chain rules it enforces.',
      },
    ],
    apiTitle: 'Verifier API',
    apiLead: 'Served from this domain. The published policy is authoritative; read it before integrating.',
    backToDocs: 'All documentation',
  },
};

const zh: Copy = {
  htmlLang: 'zh-CN',
  meta: {
    siteTitle: 'GyID 地理元 —— 用真实轨迹证明你是真人',
    description:
      'GyID 是 IETF TRIP 草案的开源实现：量化面包屑、哈希链证据、主动验证与人类存在证明（PoH）。原始 GPS 永不出端。',
  },
  nav: {
    home: '首页',
    protocol: '协议',
    architecture: '架构',
    docs: '文档',
    console: '控制台',
    about: '关于',
    github: 'GitHub',
    langSwitch: 'EN',
  },
  console: {
    title: '控制台',
    lead: '全部在浏览器本地完成。身份密钥在端上生成并用你的口令加密，永不离开这台设备。',
    tabs: { identity: '身份', collect: '采集', import: '导入', verify: '验证' },
    verifier: '验证方',
    unlocked: '已解锁（密钥仅在内存）',
    locked: '已锁定',
    lock: '锁定',
    unlock: '解锁',
    disclaimer:
      '研究原型。本站验证方使用开发密钥，请勿用于生产环境的安全决策。',
  },
  footer: {
    tagline: '证明你是真人，而不必暴露你去过哪里。',
    product: '产品',
    resources: '资源',
    legal: '法律',
    draftNote: 'IETF 互联网草案的开源实现，协议基线',
    rights: 'MIT OR Apache-2.0',
  },
  home: {
    eyebrow: '基于 IETF TRIP 草案',
    title: '用你真实走过的路，证明你是真人',
    subtitle: 'Geoyuan ID，简称 GyID',
    lead: 'GyID 是一枚无法伪造、无法买卖、也不会泄露位置的真人身份。不必上传证件、不必刷脸、不必设置密码——只要你在真实世界里持续地生活和移动。',
    ctaPrimary: '打开控制台',
    ctaSecondary: '阅读协议',
    badges: ['无需证件 · 人脸 · 密码', '原始位置永不上传', '一人一号，防女巫', '区块链上可查验'],
    featuresTitle: '为什么 GyID 不一样',
    featuresLead:
      '机器人可以一夜注册一万个账号，却伪造不出一条真实生活的轨迹：人的移动带有独特的统计物理规律，脚本批量生成的位移一眼可辨。',
    features: [
      {
        title: '只有真人能拥有',
        body: '人的位移服从幂律：移动功率谱按 1/f^α 衰减，α 落在粉噪区间。合成轨迹要么谱平坦（α≈0），要么呈现趋势漂移（α≳1.2）——单个统计量即可区分。',
        accent: 'emerald',
      },
      {
        title: '你的位置永远不上传',
        body: '每次定位在端上被量化成 H3 网格（res 10，约 15,000 m²），只有网格编号、context 摘要与签名会被传输。连验证方也无法还原你去过哪里。',
        accent: 'violet',
      },
      {
        title: '没法买、没法借、没法代持',
        body: '每次证明都绑定一次性实时挑战，只有持有轨迹的现场设备能即时应答。截图、录屏、把账号转给别人，统统无效。',
        accent: 'amber',
      },
      {
        title: '不绑姓名，也不用注册',
        body: '身份只是设备自己生成的一对密钥。服务方知道的是「一个真人」，而不是「你是谁」。',
        accent: 'pink',
      },
    ],
    flowTitle: '四步，拿到你的 Geoyuan ID',
    flowLead: '不到任何地方注册，一切在本地累积，然后被证明。',
    steps: [
      {
        n: '01',
        title: '创建身份',
        body: '密钥对在你的浏览器里生成，并用你设置的口令加密。解锁期间，seed 只存在于内存。',
      },
      {
        n: '02',
        title: '采集面包屑',
        body: '移动过程中，每次定位成为一条已签名面包屑：H3 网格、时间戳、context 摘要与上一块哈希。间隔不短于 15 分钟。',
      },
      {
        n: '03',
        title: '封装 epoch 并锚定',
        body: '每 100 条面包屑封装成一个 Merkle 根，根被登记到链上（Base）作为公开存在性证明——只含聚合值。',
      },
      {
        n: '04',
        title: '应答实时挑战',
        body: '依赖方给出 nonce，验证方通过实时通道下发挑战，你的设备签名应答。验证方随后签发绑定该 nonce 的 PoH 证书。',
      },
    ],
    engineTitle: '经典临界性引擎',
    engineLead:
      '协议规定的是确定性、可解释的统计量。GyID 逐字节实现它们作为互操作基线；AI 只能作为独立签名的附加证据。',
    engine: [
      {
        title: 'PSD 标度指数 α',
        body: '相邻 H3 网格中心间的位移构成序列。朴素 DFT 得到功率谱，log-log 回归得到 α。生物区间为 0.30–0.80，中心 0.55。',
      },
      {
        title: '截断 Lévy 拟合 β 与 κ',
        body: '位移分布 P(Δr) ∝ Δr^(−β)·e^(−Δr/κ)，按 epoch 做最大似然拟合。人类 β 通常在 1.50–1.90。',
      },
      {
        title: 'Levy–PSD 桥校验',
        body: 'α ≈ (3 − β)·g，g 期望落在 0.3–0.7。不一致往往意味着数据质量问题，或对单一统计量的对抗操纵。',
      },
      {
        title: '六分量 Hamiltonian',
        body: '空间、时间、动力学、群体共址、上下文与结构六项合成异常分，对应 NOMINAL / ELEVATED / SUSPICIOUS / CRITICAL 四级告警。',
      },
    ],
    trustTitle: '信任分与各档位含义',
    trustLead:
      'T = 40·min(n/200,1) + 30·min(unique/50,1) + 20·min(days/365,1) + 10·chain_integrity。临界测试失败会强制把 T 封顶在 50。',
    trustRows: [
      ['Bootstrap 起步', '0 – 63', '尚未计算 PSD，如实展示为「成长中」的身份。'],
      ['Provisional 暂定', '64 – 199', '可计算 PSD 但方差较大，足以声明 handle。'],
      ['Stable 稳定', '200 – 255', '方差低于 0.05，可靠正判。'],
      ['High confidence 高置信', '256+', '可用于高风险依赖方决策。'],
    ],
    stackTitle: '仓库里有什么',
    stackLead: '六个 Rust crate、一个 Solidity 合约、两个前端，全部开源。',
    stack: [
      {
        title: 'trip-core',
        body: '确定性 CBOR、Ed25519、面包屑哈希链、epoch 与 Merkle 根、活体挑战、PoH 证书、DID/TIT、临界性引擎与 EVM 锚定编码。',
        tag: 'Rust',
      },
      {
        title: 'trip-server',
        body: '验证方服务：按链规则接收证据、WebSocket 主动验证、签发 PoH、发布 .well-known 策略文档。',
        tag: 'Rust',
      },
      {
        title: 'trip-cli（gyid）',
        body: '命令行工具：身份、采集、验证、did/tit、链上锚定与人群标定。',
        tag: 'Rust',
      },
      {
        title: 'gyid-wasm + 网页控制台',
        body: '浏览器 Attester。全部密码学都是同一份 Rust 代码编译成 WebAssembly——本站直接使用它。',
        tag: 'WASM',
      },
      {
        title: 'GeoTITRegistry.sol',
        body: '链上存在性登记簿：登记公钥、锚定 epoch 根、声明 handle。不发币、不存坐标、verifier 门控。',
        tag: 'Solidity',
      },
      {
        title: 'gyid-android',
        body: '原生采集端：前台服务持续记录、断点续传，验证流程与网页端一致。',
        tag: 'Kotlin',
      },
    ],
  },
  protocol: {
    eyebrow: '协议',
    title: 'TRIP 到底规定了什么',
    lead: 'GyID 实现 IETF RATS 工作组的 draft-ayerbe-trip-protocol-04。草案刻意收窄范围：它规定证据，不规定传输、命名与锚定——那些留白正是本实现贡献 companion 草案的地方。',
    sections: [
      {
        title: '面包屑 Breadcrumb',
        body: '证据的基本单元。一个九字段的 CBOR map，用 Ed25519 签名；块哈希是完整编码的 SHA-256。',
        points: [
          'index —— 链内序号，从 0 起连续',
          'identity —— Attester 的 Ed25519 公钥',
          'timestamp —— Unix 秒',
          'h3 cell + 分辨率 —— 7 到 10，默认 10',
          'context digest —— 对 h3／时间桶／Wi-Fi／基站／IMU 分量做 SHA-256',
          'prev hash —— 上一块哈希，创世为 null',
          'meta flags —— 含探索会话标志',
          'signature —— 覆盖字段 0–7 的确定性 CBOR',
        ],
      },
      {
        title: '链规则',
        body: '让伪造变得昂贵的规则，端上与验证方执行完全相同的判定。',
        points: [
          '相邻面包屑落在同一 H3 网格必须拒绝',
          '单个网格累计上限 10 条',
          '最小间隔 15 分钟；显式的探索会话可缩短至 5 分钟',
          'index 必须连续，且每条都引用上一块哈希',
          '每条面包屑各自携带 Ed25519 签名',
        ],
      },
      {
        title: 'Epoch 与锚定',
        body: '面包屑永远不上链，它们被封进 Merkle 根，只有根被锚定。',
        points: [
          '默认每个 epoch 100 条面包屑',
          '根承诺有序集合，并记录唯一网格数',
          '根由身份密钥签名',
          '锚定目标与链无关；本实现使用 Base',
        ],
      },
      {
        title: '人类存在证明证书（PoH）',
        body: '依赖方真正消费的产物。15 个 CBOR 字段，由验证方签名，只含统计量。',
        points: [
          '身份、签发时间、epoch 数、面包屑数与唯一网格数',
          'α、β、κ、π 与临界置信度',
          '信任分与有效期',
          'RP nonce —— 必填，把证书绑定到某一次请求',
          '链头哈希 —— 必填，把它绑定到某一份确切证据',
        ],
      },
      {
        title: '主动验证 Active Verification',
        body: '本部署主推的唯一模式。预先录制的轨迹无法回答一个当时并不存在的挑战。',
        points: [
          '依赖方生成 16 字节 nonce',
          '验证方通过实时通道下发挑战',
          'Attester 把 nonce 与链头、期望序号一起签名',
          '验证方校验应答后，签发绑定该 nonce 的 PoH',
        ],
      },
      {
        title: '多验证方设计',
        body: '任何合规实现都可以用自己的 Ed25519 密钥充当验证方；Attester 可以同时提交给多个，依赖方自行选择信任谁。',
        points: [
          '协议中不存在特权验证方',
          '验证方在 well-known 地址公布自己的公钥与策略',
          '本项目的定位是「第一个公开验证方」，而不是唯一验证方',
        ],
      },
    ],
    tableTitle: '收敛区间',
    tableHead: ['面包屑数', '区间', '能支撑什么'],
    tableRows: [
      ['0 – 63', 'Bootstrap 起步', '不定义 PSD，如实展示为「成长中」的身份'],
      ['64 – 199', 'Provisional 暂定', '可计算 PSD，方差约 0.15，足以声明 handle'],
      ['200 – 255', 'Stable 稳定', '方差低于 0.05，可靠正判'],
      ['256+', 'High confidence 高置信', '可用于高风险决策'],
    ],
    note: '草案是 Internet-Draft 而非 RFC：若不续期就会过期。本项目以 draft-04 作为互操作基线固定下来，同时跟踪后续版本。',
  },
  architecture: {
    eyebrow: '架构',
    title: '各部分如何协作',
    lead: '三个角色互相通信：Attester 持有轨迹，验证方计算统计量，依赖方提出问题。跨界流动的只有聚合值。',
    layersTitle: '分层',
    layers: [
      {
        name: 'Attester 端',
        role: '设备侧 —— 持有轨迹的唯一副本',
        items: [
          '生成并加密身份密钥',
          '在离开设备之前把 GPS 量化成 H3',
          '逐条签名面包屑并维护本地链',
          '应答实时挑战、封装 epoch',
        ],
      },
      {
        name: 'Verifier 验证方',
        role: '服务侧 —— 计算统计量，永远看不到坐标',
        items: [
          '校验链规则、签名与顺序',
          '对量化后的网格运行临界性引擎',
          '通过 WebSocket 驱动主动验证',
          '按公开策略签发 PoH 证书',
        ],
      },
      {
        name: 'Relying Party 依赖方',
        role: '任何需要确认「你是真人」的一方',
        items: [
          '生成 nonce，向验证方索取证书',
          '用公开的验证方公钥校验证书签名',
          '只读取统计量——α、β、置信度、信任分',
        ],
      },
      {
        name: 'Anchor 锚定',
        role: '公开存在性证明，刻意保持最小',
        items: [
          '把「公钥 → DID 后缀」一次性登记',
          '存储 epoch Merkle 根，序号严格连续',
          '存储超过门槛的 handle 声明',
          '不发币、不存坐标、无代理合约',
        ],
      },
    ],
    repoTitle: 'Workspace 结构',
    repo: [
      { crate: 'trip-core', role: '协议数据结构、密码学、临界性引擎、锚定编码' },
      { crate: 'trip-server', role: '验证方 HTTP/WS 服务，内存存储' },
      { crate: 'trip-cli（gyid）', role: '命令行：身份、采集、验证、锚定、标定' },
      { crate: 'gyid-shared', role: 'CLI／网页／移动端共享的业务逻辑' },
      { crate: 'gyid-wasm', role: '浏览器 Attester 桥，由同一份 Rust 核心编译' },
      { crate: 'gyid-web-astro', role: '本站：门户、文档与在线控制台' },
      { crate: 'contracts', role: 'GeoTITRegistry.sol 及其编译／内存 EVM 工具链' },
      { crate: 'gyid-android', role: '原生 Android 采集端与验证客户端' },
    ],
    endpointsTitle: '验证方 API',
    endpoints: [
      { method: 'POST', path: '/v1/evidence', desc: '提交一批已签名面包屑，服务端校验链规则' },
      { method: 'GET', path: '/v1/identity/:hex', desc: '某个 attester 的链头、面包屑数与唯一网格数' },
      { method: 'POST', path: '/v1/verify', desc: '创建绑定 RP nonce 的挑战（主动验证）' },
      { method: 'WS', path: '/v1/challenge', desc: '实时通道：下发挑战、上传应答' },
      { method: 'GET', path: '/v1/poh/:challenge_id', desc: '取回某个挑战已签发的证书' },
      { method: 'GET', path: '/v1/explorer', desc: '本验证方已知全部 attester 的聚合视图' },
      { method: 'GET', path: '/.well-known/verifier.json', desc: '验证方公钥、策略门槛与公开限制' },
    ],
  },
  about: {
    eyebrow: '关于',
    title: '这个项目是什么，以及不是什么',
    lead: 'GyID（Geoyuan ID）是 IETF TRIP 草案的首个公开开源实现。它的存在是为了让规范变得具体、可测试、可被批评。',
    sections: [
      {
        title: '首先是协议实现',
        body: '价值在于逐字节的一致性与公开的证据，而不是产品功能列表。黄金测试向量、Monte Carlo 研究与人群标定都在仓库里，供其它实现对照检查。',
      },
      {
        title: '架构上的隐私',
        body: '草案禁止在任何协议消息中出现原始 GPS，本实现在最底层强制这一点。',
        points: [
          '坐标在签名之前就已在本机量化为 H3',
          '链上只存聚合值——Merkle 根与计数',
          '不涉及姓名、手机号或任何政府标识',
          '身份密钥本地生成，用用户口令加密',
        ],
      },
      {
        title: '诚实面对开放问题',
        body: '草案自己列出了未决问题，本项目不会假装已经解决。',
        points: [
          '单轨迹 ROC 与置信区间需要人群标定——见 W7 报告',
          '六分量 Hamiltonian 在统计上并不独立',
          '机器狗／无人机携带设备的攻击仍是开放研究问题',
          '自适应对手可以尝试塑造单一统计量，因此引擎从不依赖单一判据',
        ],
      },
      {
        title: '它不是什么',
        body: '不是 IETF 标准，不是带安全保证的产品，也不是识别「你是谁」的手段。',
        points: [
          '草案是 Internet-Draft，没有 IETF 背书',
          '本站验证方运行在开发密钥上',
          '尚未经过安全审计，合约仅部署在测试网',
          '它回答「这是不是一个人」，从不回答「是哪个人」',
        ],
      },
    ],
    statusTitle: '实现进度',
    statusRows: [
      ['W1 – W6', '协议核心、链规则、临界性引擎、Hamiltonian、信任分、验证方服务、WASM Attester 与演示页', '已完成'],
      ['W7', '基于 GeoLife 的人群标定 + 合成对照基线、真实 ROC / AUC、白皮书草稿', '已完成'],
      ['W8', 'did:geoyuan 与 TIT、Base 测试网 GeoTITRegistry、CLI 锚定命令', '已完成'],
      ['W9 – W12', '神经证据层、对抗竞技场、整合试点、主网与公开发布', '未开始'],
    ],
  },
  docs: {
    indexTitle: '文档',
    indexLead: '规范注解、标定报告与验证方 API —— 建议与草案对照阅读。',
    cards: [
      {
        slug: 'docs/protocol-notes',
        title: '协议实现注解',
        desc: 'draft-04 与本实现的逐字段对照，以及本项目填补的留白。',
      },
      {
        slug: 'docs/calibration',
        title: '人群标定报告',
        desc: 'α 边界的标定、合成对照族、ROC / AUC，以及这些数字能支撑与不能支撑的结论。',
      },
      {
        slug: 'docs/api',
        title: '验证方 API 参考',
        desc: '本部署暴露的每个端点、请求与响应结构，以及它执行的链规则。',
      },
    ],
    apiTitle: '验证方 API',
    apiLead: '由本站同源提供。公布的策略文档是权威来源，集成前请先阅读。',
    backToDocs: '全部文档',
  },
};

export const ui: Record<Lang, Copy> = { en, zh };

/** 全站导航（slug 供 path() 使用）。 */
export function navItems(t: Copy): NavItem[] {
  return [
    { slug: '', label: t.nav.home },
    { slug: 'protocol', label: t.nav.protocol },
    { slug: 'architecture', label: t.nav.architecture },
    { slug: 'docs', label: t.nav.docs },
    { slug: 'about', label: t.nav.about },
  ];
}

/** 控制台导航。 */
export function consoleTabs(t: Copy): NavItem[] {
  return [
    { slug: 'console', label: t.console.tabs.identity },
    { slug: 'console/collect', label: t.console.tabs.collect },
    { slug: 'console/import', label: t.console.tabs.import },
    { slug: 'console/verify', label: t.console.tabs.verify },
  ];
}
