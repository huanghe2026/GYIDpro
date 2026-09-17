// Centralized i18n content for the Geoyuan site.
// Each locale holds `meta` (SEO) plus per-page content slices.
// Page components read from `pages[locale].<page>`.

export type NavItem = { href: string; label: string };

export const nav: Record<string, NavItem[]> = {
  en: [
    { href: '/', label: 'Home' },
    { href: '/gyid', label: 'GyID' },
    { href: '/wallet', label: 'Wallet' },
    { href: '/download', label: 'Download' },
    { href: '/nft', label: 'NFT Apps' },
    { href: '/about', label: 'About' },
    { href: '/docs', label: 'Docs' },
  ],
  zh: [
    { href: '/zh', label: '首页' },
    { href: '/zh/gyid', label: 'GyID' },
    { href: '/zh/download', label: '下载' },
    { href: '/zh/nft', label: 'NFT 应用' },
    { href: '/zh/about', label: '关于' },
    { href: '/zh/docs', label: '文档' },
  ],
};

export const footer = {
  en: {
    tagline: 'Decentralized Identity for the Physical World',
    builtWith: 'Built with Rust & ❤️',
    productTitle: 'Product',
    product: [
      { label: 'GyID', href: '/gyid' },
      { label: 'Download', href: '/download' },
      { label: 'NFT Apps', href: '/nft' },
      { label: 'diliy.cn ↗', href: 'https://www.diliy.cn', external: true },
    ],
    resourceTitle: 'Resources',
    resource: [
      { label: 'Docs', href: '/docs' },
      { label: 'About', href: '/about' },
      { label: 'GitHub ↗', href: 'https://github.com/huanghe2026/GYID', external: true },
    ],
    communityTitle: 'Community',
    email: 'hangzhou62@gmail.com',
    wechat: 'WeChat: stablecoins',
    protocol: 'H3 Spatial Protocol',
    chains: 'Polygon · Aptos · Web3',
    copyright: '© 2026 Geoyuan. All rights reserved.',
    online: 'ONLINE',
    version: 'v1.0.0',
  },
  zh: {
    tagline: '物理世界的去中心化身份',
    builtWith: '用 Rust 与 ❤️ 构建',
    productTitle: '产品',
    product: [
      { label: 'GyID', href: '/zh/gyid' },
      { label: '下载', href: '/zh/download' },
      { label: 'NFT 应用', href: '/zh/nft' },
      { label: 'diliy.cn ↗', href: 'https://www.diliy.cn', external: true },
    ],
    resourceTitle: '资源',
    resource: [
      { label: '文档', href: '/zh/docs' },
      { label: '关于', href: '/zh/about' },
      { label: 'GitHub ↗', href: 'https://github.com/huanghe2026/GYID', external: true },
    ],
    communityTitle: '社区',
    email: 'hangzhou62@gmail.com',
    wechat: '微信: stablecoins',
    protocol: 'H3 空间协议',
    chains: 'Polygon · Aptos · Web3',
    copyright: '© 2026 地理元 Geoyuan. 保留所有权利。',
    online: 'ONLINE',
    version: 'v1.0.0',
  },
};

export const pages = {
  en: {
    meta: {
      title: 'Geoyuan — Decentralized Identity for the Physical World',
      description:
        'Geoyuan combines geographic location, hardware fingerprint, and blockchain to create unique, permanent decentralized identities.',
    },
    home: {
      hero: {
        badge: 'Decentralized Identity Protocol',
        titleTop: 'Your Identity,',
        titleBottom: 'Anchored in Reality',
        subtitle:
          'Geoyuan generates unique, permanent digital identities from your physical location, device hardware, and time — verified on-chain, owned by you.',
        primaryBtn: { label: '⬇ Get GyID', href: '/download' },
        walletPromo: {
          icon: '💰',
          title: 'GeoYuan Wallet',
          href: '/wallet',
          cta: 'Learn More →',
        },
      },
      stats: [
        { value: '2', label: 'Blockchains' },
        { value: '5+', label: 'Platforms' },
        { value: 'L3', label: 'Geo Precision' },
      ],
      features: {
        badge: 'CORE FEATURES',
        title: 'Built Different',
        subtitle: 'A new paradigm for digital identity that starts where you are',
        items: [
          {
            icon: '🌍',
            title: 'Geo-Anchored',
            desc: 'Three levels of geographic precision — city, region, or exact GPS coordinates — baked into your identity.',
          },
          {
            icon: '🔐',
            title: 'Hardware-Bound',
            desc: "Your device's unique fingerprint ensures identities are truly tied to physical reality, not just accounts.",
          },
          {
            icon: '⛓️',
            title: 'On-Chain Verified',
            desc: 'Anchored to Polygon and Aptos blockchains. Immutable, permanent, trustless verification.',
          },
          {
            icon: '⬡',
            title: 'H3 Grid System',
            desc: "Uses Uber's H3 hexagonal grid for spatial indexing — enabling geo-based NFT ownership and discovery.",
          },
          {
            icon: '🔗',
            title: 'Multi-Device',
            desc: 'Link multiple devices in a primary-subordinate tree structure under one identity.',
          },
          {
            icon: '🛡️',
            title: 'Self-Sovereign',
            desc: 'No central authority. No server. Your GyID is generated locally and belongs to you forever.',
          },
        ],
      },
      how: {
        badge: 'PROTOCOL',
        title: 'How It Works',
        steps: [
          { n: '01', title: 'Collect', desc: 'Gather hardware fingerprint, GPS/WiFi/IP location, and timestamp in parallel.' },
          { n: '02', title: 'Hash', desc: 'Combine inputs through a multi-dimensional cryptographic hash algorithm.' },
          { n: '03', title: 'Encode', desc: 'Encode the result in Base58 to produce a human-readable GyID string.' },
          { n: '04', title: 'Anchor', desc: 'Optionally anchor the identity hash to Polygon or Aptos blockchain.' },
        ],
      },
      liveApp: {
        badge: 'LIVE APP',
        title: 'Explore diliy.cn — Geo NFT Platform',
        desc: 'Claim your piece of the world. Every location is an H3 hexagon NFT, verified by GyID, permanently owned on-chain.',
        tags: ['H3 Hexagon', 'NFT', 'GyID Auth', 'Blockchain'],
        primaryBtn: { label: 'Visit diliy.cn ↗', href: 'https://www.diliy.cn', external: true },
        secondaryBtn: { label: 'Learn More', href: '/nft' },
      },
      getStarted: {
        badge: 'GET STARTED',
        title: 'Get Your GyID Today',
        subtitle: 'Available for Windows, macOS, Linux, Android, iOS, and Web.',
        primaryBtn: { label: '⬇ Download GyID', href: '/download' },
        secondaryBtn: { label: 'View Docs →', href: '/docs' },
      },
    },

    gyid: {
      hero: {
        badge: 'IDENTITY PROTOCOL',
        title: 'What is GyID?',
        subtitle: 'The Identity Layer for Geoyuan',
        desc: 'GyID (Geoyuan Identity) is a decentralized identity system that generates unique, permanent IDs from multiple dimensions: hardware fingerprint, geographic location, timestamp, and avatar hash.',
      },
      how: {
        badge: 'PROTOCOL',
        title: 'How It Works',
        steps: [
          { n: '01', title: 'Collect', desc: 'Gather hardware fingerprint, GPS/WiFi/IP location, and timestamp in parallel.' },
          { n: '02', title: 'Hash', desc: 'Combine inputs through a multi-dimensional cryptographic hash algorithm.' },
          { n: '03', title: 'Encode', desc: 'Encode the result in Base58 to produce a human-readable GyID string.' },
          { n: '04', title: 'Anchor', desc: 'Optionally anchor the identity hash to Polygon or Aptos blockchain.' },
        ],
      },
      props: [
        { title: 'Permanent', desc: 'Once generated, a GyID is immutable and cannot be deleted.' },
        { title: 'Offline-First', desc: 'Works without internet. GPS and hardware fingerprint alone can generate a valid ID.' },
        { title: 'Multi-Chain', desc: 'Supports Polygon PoS (EVM) and Aptos (Move) for on-chain anchoring.' },
        { title: 'H3 Spatial', desc: 'Built-in H3 hexagonal grid encoding for spatial identity and NFT geofencing.' },
      ],
      h3: {
        badge: 'HEXAGONAL GRID',
        title: 'H3 Spatial Indexing',
        icon: '⬡',
        desc: "GyID uses Uber's open-source H3 hexagonal geospatial indexing system to partition the Earth's surface into hierarchical hexagonal cells. Each cell has a unique identifier for location encoding and spatial indexing.",
        levels: [
          { name: 'L1 City', res: 'Res 3-4', area: '110-1100 km²' },
          { name: 'L2 District', res: 'Res 6-7', area: '4-10 km²' },
          { name: 'L3 Exact', res: 'Res 9-10', area: '0.1-0.4 km²' },
        ],
        docBtn: { label: '📖 View H3 Protocol Documentation', href: '/docs' },
      },
      format: {
        badge: 'GYID FORMAT',
        title: 'GyID Format Breakdown',
        example: 'GY·5xK9mN2pQ8rT4wV·2601032158341A·8f3c',
        note: 'Example GyID (illustrative only)',
        parts: [
          { part: 'GY·', title: 'Prefix', desc: 'Geoyuan protocol marker' },
          { part: '5xK9...4wV', title: 'Multi-dim Hash', desc: 'Hardware + location + time hash' },
          { part: '2601...1A·8f3c', title: 'Timestamp+Rand', desc: 'UTC ms + 16bit random' },
        ],
        primaryBtn: { label: 'Download Now', href: '/download' },
        secondaryBtn: { label: 'Read the Docs', href: '/docs' },
      },
    },

    wallet: {
      hero: {
        badge: 'GEOCOIN WALLET',
        title: 'GeoYuan Wallet',
        subtitle: 'Your GeoCoin Wallet, Powered by Identity',
        desc: 'The GeoYuan Wallet is a decentralized wallet tied to your GyID. Each GeoCoin represents a geo-anchored digital asset minted from a real-world photo with GPS location.',
      },
      features: [
        { icon: '💰', title: 'GeoCoin Minting', desc: 'Mint one unique GeoCoin from each geo-tagged photo. Each coin is bound to its location and timestamp.' },
        { icon: '🔄', title: 'P2P Transfers', desc: 'Transfer GeoCoins between wallets via the libp2p peer-to-peer network. All transactions are signed with your Ed25519 key.' },
        { icon: '🔗', title: 'Multi-Device Sync', desc: 'Link multiple devices under one GyID and sync your wallet across all of them automatically.' },
        { icon: '⬡', title: 'Spatial Discovery', desc: 'Discover GeoCoins minted near your location using H3 hexagonal spatial indexing.' },
      ],
      how: {
        badge: 'PROTOCOL',
        title: 'How It Works',
        steps: [
          { n: '01', title: 'Generate GyID', desc: 'Create your decentralized identity using hardware fingerprint, GPS, and timestamp.' },
          { n: '02', title: 'Take a Photo', desc: 'Capture a geo-tagged photo at any location. The EXIF GPS data anchors your GeoCoin to reality.' },
          { n: '03', title: 'Mint GeoCoin', desc: 'Your photo is hashed and combined with location data to mint a unique, non-fungible GeoCoin.' },
          { n: '04', title: 'Transfer & Trade', desc: 'Send GeoCoins to other GyIDs via P2P network. Each transfer is cryptographically signed.' },
        ],
      },
      p2p: {
        badge: 'P2P NETWORK',
        title: 'P2P Network',
        icon: '🌐',
        desc: 'GeoYuan uses a libp2p-based peer-to-peer network for decentralized wallet operations. Nodes discover each other via mDNS (local network) and Kademlia DHT (global network).',
        points: [
          'Decentralized — no central server required',
          'Encrypted — all traffic is end-to-end encrypted',
          'Resilient — network auto-heals when peers go offline',
          'Fast — local transfers via mDNS, global via DHT relay',
        ],
      },
      arch: {
        badge: 'ARCHITECTURE',
        title: 'System Architecture',
        ascii: `┌─────────────────────────────────────────┐
│        GeoYuan Desktop (Slint)        │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│           geoyuan-core (Rust)            │
│   GyID · Wallet · P2P · Crypto · Geo     │
└────┬──────────┬──────────┬─────────────┘
     │          │          │
  📦 Storage  🌐 P2P     ⛓️ Blockchain
   redb+SQLite  libp2p     Polygon · Aptos
        (mDNS+Kademlia)`,
        footer: 'Ed25519 Signatures · BLAKE3 Hashes · H3 Spatial Grid',
        primaryBtn: { label: '↓ Download GeoYuan', href: '/download' },
        secondaryBtn: { label: '📖 View Docs', href: '/docs' },
      },
    },

    download: {
      hero: {
        badge: '⬇ DOWNLOAD',
        title: 'Download GyID',
        subtitle: 'Available for all major platforms',
      },
      gui: {
        title: 'GyID GUI',
        desc: 'Desktop application with full GUI. Generate, manage, export, and verify your GyIDs.',
        platforms: [
          {
            icon: '🪟',
            name: 'Windows',
            version: 'v0.1.0 · 19 MB',
            downloads: [
              { label: 'GUI Application (.exe)', href: '#' },
              { label: 'CLI Installer (.exe)', href: '#' },
              { label: 'CLI Binary (.exe)', href: '#' },
            ],
            guideTitle: 'Install Guide',
            guide: ['1. Download the .exe installer', '2. Run as Administrator', '3. Follow the setup wizard', '4. Launch "GyID" from Start Menu'],
          },
          {
            icon: '🐧',
            name: 'Linux',
            version: 'v0.1.0 · 8 MB',
            downloads: [
              { label: 'x86_64 Binary', href: '#' },
              { label: 'ARM64 Binary', href: '#' },
            ],
            guideTitle: 'Install Guide',
            guide: [
              '# Install via cargo',
              'cargo install geoyuan-cli',
              '# Or download binary from GitHub Releases',
              'curl -L https://github.com/huanghe2026/GYID/releases/latest/download/geoyuan-cli-v0.1.0-linux-x64 -o geoyuan',
              'chmod +x geoyuan',
              'sudo mv geoyuan /usr/local/bin/',
            ],
          },
          {
            icon: '🍎',
            name: 'macOS',
            version: 'v0.1.0 · 10 MB',
            comingSoon: true,
            downloads: [
              { label: 'Apple Silicon (arm64)', href: '#' },
              { label: 'Intel (x86_64)', href: '#' },
            ],
            guideTitle: 'Install Guide',
            guide: [
              '# Install via Homebrew (coming soon)',
              'brew install geoyuan/tap/geoyuan',
              '# Or download binary from GitHub Releases',
              'curl -L https://github.com/huanghe2026/GYID/releases/latest/download/geoyuan-cli-v0.1.0-macos-arm64 -o geoyuan',
              'chmod +x geoyuan',
              'sudo mv geoyuan /usr/local/bin/',
            ],
          },
        ],
        note: '⚠ Windows binaries are ready! Linux / macOS will be available via GitHub Releases soon.',
      },
      cli: {
        title: 'GyID CLI',
        desc: 'Command-line tool for generating and managing GyIDs. Perfect for developers and automation.',
        cargoTitle: '🦀 Install via Cargo (Recommended)',
        cargo: ['cargo install geoyuan-cli', '# Verify installation', 'geoyuan --version', 'geoyuan 0.1.0'],
        usageTitle: '⚡ Basic Usage',
        usage: [
          'geoyuan identity generate --photo photo.jpg',
          'geoyuan identity generate --testnet',
          'geoyuan identity verify <GYID>',
          'geoyuan identity info <GYID>',
          'geoyuan wallet show',
          'geoyuan consensus status',
          'geoyuan p2p start --port 0',
        ],
        buildTitle: '🔧 Build from Source',
        build: [
          'git clone https://github.com/huanghe2026/GYID.git',
          'cd GYID',
          'cargo build --release -p geoyuan-cli',
          './target/release/geoyuan --version',
        ],
      },
      sdk: {
        title: 'GyID SDK',
        desc: 'Integrate GyID into your applications. Available for multiple platforms.',
        items: [
          { icon: '🦀', name: 'Rust (Cargo)', file: 'Cargo.toml', code: 'gyid-core = "0.1.0"', comingSoon: true },
          { icon: '🌐', name: 'Web (WASM)', file: 'terminal', code: 'npm install @geoyuan/gyid-wasm', comingSoon: true },
          { icon: '🤖', name: 'Android (Kotlin)', file: 'build.gradle', code: 'implementation("com.geoyuan:gyid-android:0.1.0")', comingSoon: true },
          { icon: '🍏', name: 'iOS (Swift)', file: 'Package.swift', code: '.package(url: "https://github.com/huanghe2026/GYID", from: "0.1.0")', comingSoon: true },
          { icon: '⚙️', name: 'C FFI', file: 'main.c', code: '#include "gyid.h"', comingSoon: true },
        ],
        quickTitle: 'Quick Integration (Rust)',
        quickCode: `use gyid_core::{GyIdGenerator, GeneratorConfig};

#[tokio::main]
async fn main() {
    let config = GeneratorConfig::default();
    let generator = GyIdGenerator::new(config);
    let identity = generator.generate().await.unwrap();
    println!("GyID: {}", identity.id);
    println!("Location: {:?}", identity.location);
}`,
      },
    },

    nft: {
      hero: {
        badge: '⬡ NFT ECOSYSTEM',
        title: 'NFT Applications',
        subtitle: 'Geography meets the digital asset economy',
        desc: "Geoyuan's H3-based geographic system enables a new generation of location-anchored NFTs. Real-world locations become digital assets.",
      },
      live: {
        badge: 'LIVE',
        title: 'Geographic NFT Platform',
        host: 'diliy.cn',
        name: 'Diliy',
        desc: 'The first Geoyuan-powered NFT platform. Claim, trade, and own geographic tiles represented as H3 hexagons on the blockchain. Every piece of land, every location — a unique digital asset.',
        tags: ['H3 Hexagon NFTs', 'Geographic Ownership', 'Location-Based Discovery', 'GyID Integration'],
        primaryBtn: { label: 'Visit diliy.cn ↗', href: 'https://www.diliy.cn', external: true },
        secondaryBtn: { label: 'View Roadmap', href: '#roadmap' },
      },
      roadmap: {
        badge: 'ROADMAP',
        title: 'NFT Ecosystem Roadmap',
        items: [
          { date: 'Q2 2026', title: 'diliy.cn Launch', status: 'ACTIVE', desc: 'Full platform launch with H3 geo-NFT minting and trading' },
          { date: 'Q3 2026', title: 'GyID NFT Passport', status: 'PLANNED', desc: 'Mint your GyID as a soulbound NFT — a verifiable digital passport' },
          { date: 'Q4 2026', title: 'Geo-Finance', status: 'PLANNED', desc: 'Location-based DeFi: yield farming tied to real-world geographic data' },
          { date: '2027', title: 'AR Integration', status: 'FUTURE', desc: 'Augmented reality layer for on-site NFT discovery and interaction' },
        ],
      },
      build: {
        badge: 'BUILD WITH US',
        title: 'Build the Next Geo NFT App',
        desc: 'The GyID SDK provides full geo-identity and H3 spatial indexing capabilities to help you build location-anchored NFT apps quickly.',
        primaryBtn: { label: 'View Developer Docs', href: '/docs' },
        secondaryBtn: { label: 'Download SDK', href: '/download' },
      },
    },

    about: {
      hero: {
        badge: 'ABOUT',
        title: 'About Geoyuan',
        subtitle: '地理元 — Where Geography Meets Web3',
      },
      vision: {
        n: '01',
        title: 'Vision',
        desc: 'We believe your physical location is a fundamental part of who you are. Geoyuan bridges the gap between the physical world and digital identity — creating a new layer of trust that is rooted in reality.',
      },
      mission: {
        n: '02',
        title: 'Mission',
        desc: 'Build open, decentralized infrastructure for geographic identity — enabling individuals to own their location data, monetize their physical presence, and participate in the geo-anchored digital economy.',
      },
      tech: {
        badge: 'TECH',
        title: 'Technology Stack',
        items: [
          { n: '01', title: 'Rust (core engine)' },
          { n: '02', title: 'H3 Hexagonal Grid (Uber)' },
          { n: '03', title: 'Polygon PoS & Aptos (blockchain)' },
          { n: '04', title: 'Ed25519 / secp256k1 (cryptography)' },
          { n: '05', title: 'SQLite / redb (storage)' },
          { n: '06', title: 'libp2p (P2P network)' },
          { n: '07', title: 'Slint (Desktop GUI)' },
        ],
      },
      contact: {
        badge: 'CONTACT',
        title: 'Contact',
        items: [
          { icon: '⌘', title: 'GitHub', value: 'github.com/huanghe2026/GYID', href: 'https://github.com/huanghe2026/GYID', external: true },
          { icon: '✉', title: 'Email', value: 'hangzhou62@gmail.com', href: 'mailto:hangzhou62@gmail.com' },
          { icon: '💬', title: 'WeChat', value: 'stablecoins' },
        ],
      },
    },

    docs: {
      hero: {
        badge: 'DOCS',
        title: 'Documentation',
        subtitle: 'Everything you need to build with GyID',
      },
      quickstart: {
        title: 'Quickstart',
        desc: 'Generate your first GyID in 5 minutes',
        code: ['cargo install geoyuan-cli', 'geoyuan identity generate --photo photo.jpg', '✓ Your GyID has been generated and saved'],
        btn: { label: 'Get Started', href: '/download' },
      },
      sections: [
        { n: '01', title: 'Getting Started', desc: 'Install GyID and generate your first identity in minutes.', cta: 'Read docs →', href: '#' },
        { n: '02', title: 'CLI Reference', desc: 'Complete command reference for the GyID command-line tool.', cta: 'Read docs →', href: '#' },
        { n: '03', title: 'SDK Integration', desc: 'Integrate GyID into your Android, iOS, or Web application.', cta: 'Read docs →', href: '#' },
        { n: '04', title: 'Blockchain Anchoring', desc: 'Anchor your GyID to Polygon or Aptos blockchain.', cta: 'Read docs →', href: '#' },
        { n: '05', title: 'H3 Grid API', desc: 'Work with geographic hexagons and spatial identity.', cta: 'Read docs →', href: '#' },
        { n: '06', title: 'NFT Development', desc: 'Build location-anchored NFT applications with GyID.', cta: 'Read docs →', href: '#' },
      ],
      wip: {
        badge: 'DOCS IN PROGRESS',
        desc: 'Full documentation is being written. Follow GitHub for the latest updates.',
        btn: { label: 'GitHub Repository ↗', href: 'https://github.com/huanghe2026/GYID', external: true },
      },
    },

    notFound: {
      code: '404',
      title: 'PAGE NOT FOUND',
      desc: 'This location is not on our map',
      btn: { label: 'Return Home', href: '/' },
    },
  },

  zh: {
    meta: {
      title: '地理元 — 物理世界的去中心化身份',
      description:
        'Geoyuan 从你的物理位置、设备硬件和时间生成唯一、永久的数字身份——链上验证，完全由你掌控。',
    },
    home: {
      hero: {
        badge: '去中心化身份协议',
        titleTop: '你的身份，',
        titleBottom: '根植于现实世界',
        subtitle:
          'Geoyuan 从你的物理位置、设备硬件和时间生成唯一、永久的数字身份——链上验证，完全由你掌控。',
        primaryBtn: { label: '⬇ 获取 GyID', href: '/zh/download' },
        walletPromo: {
          icon: '💰',
          title: 'GeoYuan 钱包',
          href: '/zh/wallet',
          cta: '了解更多 →',
        },
      },
      stats: [
        { value: '2', label: '区块链' },
        { value: '5+', label: '支持平台' },
        { value: 'L3', label: '地理精度' },
      ],
      features: {
        badge: '核心特性',
        title: '与众不同',
        subtitle: '以你所在的位置为起点，构建全新的数字身份范式',
        items: [
          { icon: '🌍', title: '地理锚定', desc: '三级地理精度——城市、区域或精确 GPS 坐标——永久编码在你的身份中。' },
          { icon: '🔐', title: '硬件绑定', desc: '设备唯一指纹确保身份真正绑定到物理现实，而非仅仅是账号。' },
          { icon: '⛓️', title: '链上验证', desc: '锚定 Polygon 和 Aptos 区块链。不可篡改，永久留存，无需信任中间方。' },
          { icon: '⬡', title: 'H3 六边形网格', desc: '使用 Uber H3 六边形网格进行空间索引——支持基于地理的 NFT 归属与发现。' },
          { icon: '🔗', title: '多设备关联', desc: '以主从树状结构将多个设备关联到同一身份下。' },
          { icon: '🛡️', title: '自主主权', desc: '无中心机构，无服务器。GyID 在本地生成，永远属于你。' },
        ],
      },
      how: {
        badge: '工作原理',
        title: '工作原理',
        steps: [
          { n: '01', title: '采集', desc: '并行采集硬件指纹、GPS/WiFi/IP 位置和时间戳。' },
          { n: '02', title: '哈希', desc: '通过多维度加密哈希算法组合所有输入数据。' },
          { n: '03', title: '编码', desc: '将结果编码为 Base58 格式，生成可读的 GyID 字符串。' },
          { n: '04', title: '锚定', desc: '可选择将身份哈希锚定到 Polygon 或 Aptos 区块链。' },
        ],
      },
      liveApp: {
        badge: '已上线',
        title: '探索地利 —— 地理 NFT 平台',
        desc: '认领属于你的地理区块。每一块土地都是 H3 六边形 NFT，由 GyID 验证，链上永久归属。',
        tags: ['H3 六边形', 'NFT', 'GyID 认证', '区块链'],
        primaryBtn: { label: '访问 diliy.cn ↗', href: 'https://www.diliy.cn', external: true },
        secondaryBtn: { label: '了解更多', href: '/zh/nft' },
      },
      getStarted: {
        badge: '立即开始',
        title: '立即获取 GyID',
        subtitle: 'Windows、macOS、Linux，以及 Android、iOS 和 Web 全平台支持。',
        primaryBtn: { label: '⬇ 下载 GyID', href: '/zh/download' },
        secondaryBtn: { label: '查看文档 →', href: '/zh/docs' },
      },
    },

    gyid: {
      hero: {
        badge: '身份协议',
        title: '什么是 GyID？',
        subtitle: 'Geoyuan 的身份层',
        desc: 'GyID（Geoyuan Identity）是一个去中心化身份系统，从多个维度生成唯一、永久的 ID：硬件指纹、地理位置、时间戳和头像哈希。',
      },
      how: {
        badge: '工作原理',
        title: '工作原理',
        steps: [
          { n: '01', title: '采集', desc: '并行采集硬件指纹、GPS/WiFi/IP 位置和时间戳。' },
          { n: '02', title: '哈希', desc: '通过多维度加密哈希算法组合所有输入数据。' },
          { n: '03', title: '编码', desc: '将结果编码为 Base58 格式，生成可读的 GyID 字符串。' },
          { n: '04', title: '锚定', desc: '可选择将身份哈希锚定到 Polygon 或 Aptos 区块链。' },
        ],
      },
      props: [
        { title: '永久有效', desc: '一旦生成，GyID 不可变更，无法删除。' },
        { title: '离线优先', desc: '无需联网即可运行。仅靠 GPS 和硬件指纹就能生成有效 ID。' },
        { title: '多链支持', desc: '支持 Polygon PoS（EVM）和 Aptos（Move）链上锚定。' },
        { title: 'H3 空间索引', desc: '内置 H3 六边形网格编码，支持空间身份和 NFT 地理围栏。' },
      ],
      h3: {
        badge: '六边形网格',
        title: 'H3 空间索引系统',
        icon: '⬡',
        desc: 'GyID 使用 Uber 开源的 H3 六边形地理网格系统，将地球表面划分为层次化的六边形单元格。每个单元格都有唯一标识符，用于位置编码和空间索引。',
        levels: [
          { name: 'L1 城市级', res: '分辨率 3-4', area: '110-1100 km²' },
          { name: 'L2 区域级', res: '分辨率 6-7', area: '4-10 km²' },
          { name: 'L3 精确级', res: '分辨率 9-10', area: '0.1-0.4 km²' },
        ],
        docBtn: { label: '📖 查看完整 H3 协议文档', href: '/zh/docs' },
      },
      format: {
        badge: 'GyID 格式',
        title: 'GyID 格式解析',
        example: 'GY·5xK9mN2pQ8rT4wV·2601032158341A·8f3c',
        note: '示例 GyID（仅用于说明）',
        parts: [
          { part: 'GY·', title: '前缀', desc: '标识 Geoyuan 协议' },
          { part: '5xK9...4wV', title: '多维哈希', desc: '硬件+位置+时间哈希' },
          { part: '2601...8f3c', title: '时间戳+随机', desc: 'UTC毫秒 + 16bit随机' },
        ],
        primaryBtn: { label: '立即下载', href: '/zh/download' },
        secondaryBtn: { label: '查看文档', href: '/zh/docs' },
      },
    },

    wallet: {
      hero: {
        badge: 'GEOCOIN 钱包',
        title: 'GeoYuan 钱包',
        subtitle: '你的 GeoCoin 钱包，由身份驱动',
        desc: 'GeoYuan 钱包是与你的 GyID 绑定的去中心化钱包。每个 GeoCoin（地理元）代表一个由带 GPS 位置的真实照片铸造的地理锚定数字资产。',
      },
      features: [
        { icon: '💰', title: 'GeoCoin 铸造', desc: '每张带地理标记的照片可铸造一个唯一的 GeoCoin。每个代币与它的位置和时间戳永久绑定。' },
        { icon: '🔄', title: 'P2P 转账', desc: '通过 libp2p 点对点网络在钱包之间转账 GeoCoin。所有交易均使用你的 Ed25519 密钥签名。' },
        { icon: '🔗', title: '多设备同步', desc: '将多个设备关联到同一 GyID 下，钱包在所有设备上自动同步。' },
        { icon: '⬡', title: '空间发现', desc: '使用 H3 六边形空间索引发现你附近位置铸造的 GeoCoin。' },
      ],
      how: {
        badge: '协议流程',
        title: '工作原理',
        steps: [
          { n: '01', title: '生成 GyID', desc: '使用硬件指纹、GPS 和时间戳创建你的去中心化身份。' },
          { n: '02', title: '拍照', desc: '在任何位置拍摄带地理标记的照片。EXIF GPS 数据将你的 GeoCoin 锚定到现实。' },
          { n: '03', title: '铸造 GeoCoin', desc: '照片哈希与位置数据结合，铸造出唯一的、不可替代的 GeoCoin。' },
          { n: '04', title: '转账与交易', desc: '通过 P2P 网络将 GeoCoin 发送给其他 GyID。每次转账都经过密码学签名。' },
        ],
      },
      p2p: {
        badge: 'P2P 网络',
        title: 'P2P 网络',
        icon: '🌐',
        desc: 'GeoYuan 使用基于 libp2p 的点对点网络实现去中心化钱包操作。节点通过 mDNS（局域网）和 Kademlia DHT（全球网络）发现彼此。',
        points: ['去中心化 — 无需中心服务器', '加密传输 — 所有流量端到端加密', '弹性自愈 — 节点离线时网络自动恢复', '快速传输 — 局域网通过 mDNS，广域网通过 DHT 中继'],
      },
      arch: {
        badge: '系统架构',
        title: '系统架构',
        ascii: `┌─────────────────────────────────────────┐
│       GeoYuan 桌面应用 (Slint)       │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│          geoyuan-core (Rust)            │
│   GyID · 钱包 · P2P · 加密 · 地理     │
└────┬──────────┬──────────┬─────────────┘
     │          │          │
  📦 存储层    🌐 P2P     ⛓️ 区块链
  redb+SQLite  libp2p    Polygon · Aptos
        (mDNS+Kademlia)`,
        footer: 'Ed25519 签名 · BLAKE3 哈希 · H3 空间网格',
        primaryBtn: { label: '↓ 下载 GeoYuan', href: '/zh/download' },
        secondaryBtn: { label: '📖 查看文档', href: '/zh/docs' },
      },
    },

    download: {
      hero: {
        badge: '⬇ 下载',
        title: '下载 GyID',
        subtitle: '支持所有主流平台',
      },
      gui: {
        title: 'GyID GUI',
        desc: '完整图形界面桌面应用。生成、管理、导出和验证你的 GyID。',
        platforms: [
          {
            icon: '🪟',
            name: 'Windows',
            version: 'v0.1.0 · 19 MB',
            downloads: [
              { label: 'GUI 应用程序 (.exe)', href: '#' },
              { label: 'CLI 安装包 (.exe)', href: '#' },
              { label: 'CLI 二进制 (.exe)', href: '#' },
            ],
            guideTitle: '安装指南',
            guide: ['1. 下载 .exe 安装包', '2. 以管理员身份运行', '3. 按照安装向导操作', '4. 从开始菜单启动 "GyID"'],
          },
          {
            icon: '🐧',
            name: 'Linux',
            version: 'v0.1.0 · 8 MB',
            downloads: [
              { label: 'x86_64 二进制', href: '#' },
              { label: 'ARM64 二进制', href: '#' },
            ],
            guideTitle: '安装指南',
            guide: [
              '# 通过 cargo 安装',
              'cargo install geoyuan-cli',
              '# 或从 GitHub Releases 下载二进制文件',
              'curl -L https://github.com/huanghe2026/GYID/releases/latest/download/geoyuan-cli-v0.1.0-linux-x64 -o geoyuan',
              'chmod +x geoyuan',
              'sudo mv geoyuan /usr/local/bin/',
            ],
          },
          {
            icon: '🍎',
            name: 'macOS',
            version: 'v0.1.0 · 10 MB',
            comingSoon: true,
            downloads: [
              { label: 'Apple Silicon (arm64)', href: '#' },
              { label: 'Intel (x86_64)', href: '#' },
            ],
            guideTitle: '安装指南',
            guide: [
              '# 通过 Homebrew 安装（即将上线）',
              'brew install geoyuan/tap/geoyuan',
              '# 或从 GitHub Releases 下载二进制文件',
              'curl -L https://github.com/huanghe2026/GYID/releases/latest/download/geoyuan-cli-v0.1.0-macos-arm64 -o geoyuan',
              'chmod +x geoyuan',
              'sudo mv geoyuan /usr/local/bin/',
            ],
          },
        ],
        note: '⚠ Windows 二进制已就绪！Linux / macOS 即将通过 GitHub Releases 发布。',
      },
      cli: {
        title: 'GyID CLI',
        desc: '用于生成和管理 GyID 的命令行工具，适合开发者和自动化场景。',
        cargoTitle: '🦀 通过 Cargo 安装（推荐）',
        cargo: ['cargo install geoyuan-cli', '# 验证安装', 'geoyuan --version', 'geoyuan 0.1.0'],
        usageTitle: '⚡ 基础用法',
        usage: [
          'geoyuan identity generate --photo photo.jpg',
          'geoyuan identity generate --testnet',
          'geoyuan identity verify <GYID>',
          'geoyuan identity info <GYID>',
          'geoyuan wallet show',
          'geoyuan consensus status',
          'geoyuan p2p start --port 0',
        ],
        buildTitle: '🔧 从源码编译',
        build: ['git clone https://github.com/huanghe2026/GYID.git', 'cd GYID', 'cargo build --release -p geoyuan-cli', './target/release/geoyuan --version'],
      },
      sdk: {
        title: 'GyID SDK',
        desc: '将 GyID 集成到你的应用中，支持多个平台。',
        items: [
          { icon: '🦀', name: 'Rust (Cargo)', file: 'Cargo.toml', code: 'gyid-core = "0.1.0"', comingSoon: true },
          { icon: '🌐', name: 'Web (WASM)', file: 'terminal', code: 'npm install @geoyuan/gyid-wasm', comingSoon: true },
          { icon: '🤖', name: 'Android (Kotlin)', file: 'build.gradle', code: 'implementation("com.geoyuan:gyid-android:0.1.0")', comingSoon: true },
          { icon: '🍏', name: 'iOS (Swift)', file: 'Package.swift', code: '.package(url: "https://github.com/huanghe2026/GYID", from: "0.1.0")', comingSoon: true },
          { icon: '⚙️', name: 'C FFI', file: 'main.c', code: '#include "gyid.h"', comingSoon: true },
        ],
        quickTitle: '快速集成（Rust）',
        quickCode: `use gyid_core::{GyIdGenerator, GeneratorConfig};

#[tokio::main]
async fn main() {
    let config = GeneratorConfig::default();
    let generator = GyIdGenerator::new(config);
    let identity = generator.generate().await.unwrap();
    println!("GyID: {}", identity.id);
    println!("Location: {:?}", identity.location);
}`,
      },
    },

    nft: {
      hero: {
        badge: '⬡ NFT 生态',
        title: 'NFT 应用',
        subtitle: '地理位置遇见数字资产经济',
        desc: 'Geoyuan 的 H3 地理系统开创了新一代位置锚定 NFT。现实世界的位置成为数字资产。',
      },
      live: {
        badge: '已上线',
        title: '地理 NFT 平台',
        host: 'diliy.cn',
        name: '地利',
        desc: '首个 Geoyuan 赋能的 NFT 平台。在区块链上以 H3 六边形认领、交易和拥有地理区块。每一块土地、每一个位置——都是唯一的数字资产。',
        tags: ['H3 六边形 NFT', '地理归属权', '基于位置的发现', 'GyID 集成'],
        primaryBtn: { label: '访问 diliy.cn ↗', href: 'https://www.diliy.cn', external: true },
        secondaryBtn: { label: '查看路线图', href: '#roadmap' },
      },
      roadmap: {
        badge: '路线图',
        title: 'NFT 生态路线图',
        items: [
          { date: '2026 Q2', title: 'diliy.cn 上线', status: '进行中', desc: 'H3 地理 NFT 铸造与交易平台全面上线' },
          { date: '2026 Q3', title: 'GyID NFT 护照', status: '计划中', desc: '将你的 GyID 铸造为灵魂绑定 NFT——可验证的数字护照' },
          { date: '2026 Q4', title: '地理金融', status: '计划中', desc: '位置驱动的 DeFi：与真实地理数据挂钩的流动性挖矿' },
          { date: '2027', title: 'AR 集成', status: '未来', desc: '增强现实层，支持现场 NFT 发现与互动' },
        ],
      },
      build: {
        badge: '与我们共建',
        title: '构建下一个地理 NFT 应用',
        desc: 'GyID SDK 提供完整的地理身份和 H3 空间索引能力，帮你快速构建位置锚定的 NFT 应用。',
        primaryBtn: { label: '查看开发文档', href: '/zh/docs' },
        secondaryBtn: { label: '下载 SDK', href: '/zh/download' },
      },
    },

    about: {
      hero: {
        badge: '关于我们',
        title: '关于地理元',
        subtitle: '地理元 — 地理遇见 Web3',
      },
      vision: {
        n: '01',
        title: '愿景',
        desc: '我们相信，你所在的物理位置是你身份认同的基本组成部分。Geoyuan 弥合物理世界与数字身份之间的鸿沟——构建植根于现实的全新信任层。',
      },
      mission: {
        n: '02',
        title: '使命',
        desc: '构建开放、去中心化的地理身份基础设施——让个人掌控自己的位置数据，从物理存在中获益，参与地理锚定的数字经济。',
      },
      tech: {
        badge: '技术栈',
        title: '技术栈',
        items: [
          { n: '01', title: 'Rust（核心引擎）' },
          { n: '02', title: 'H3 六边形网格（Uber）' },
          { n: '03', title: 'Polygon PoS & Aptos（区块链）' },
          { n: '04', title: 'Ed25519 / secp256k1（密码学）' },
          { n: '05', title: 'SQLite / redb（存储层）' },
          { n: '06', title: 'libp2p（P2P 网络）' },
          { n: '07', title: 'Slint（桌面 GUI）' },
        ],
      },
      contact: {
        badge: '联系我们',
        title: '联系方式',
        items: [
          { icon: '⌘', title: 'GitHub', value: 'github.com/huanghe2026/GYID', href: 'https://github.com/huanghe2026/GYID', external: true },
          { icon: '✉', title: '邮件', value: 'hangzhou62@gmail.com', href: 'mailto:hangzhou62@gmail.com' },
          { icon: '💬', title: '微信', value: 'stablecoins' },
        ],
      },
    },

    docs: {
      hero: {
        badge: '文档',
        title: '开发文档',
        subtitle: '使用 GyID 构建所需的一切',
      },
      quickstart: {
        title: '快速开始',
        desc: '5 分钟生成你的第一个 GyID',
        code: ['cargo install geoyuan-cli', 'geoyuan identity generate --photo photo.jpg', '✓ 你的 GyID 已生成并保存'],
        btn: { label: '立即开始', href: '/zh/download' },
      },
      sections: [
        { n: '01', title: '快速入门', desc: '安装 GyID 并在几分钟内生成你的第一个身份。', cta: '查看文档 →', href: '#' },
        { n: '02', title: 'CLI 参考', desc: 'GyID 命令行工具的完整命令参考。', cta: '查看文档 →', href: '#' },
        { n: '03', title: 'SDK 集成', desc: '将 GyID 集成到 Android、iOS 或 Web 应用中。', cta: '查看文档 →', href: '#' },
        { n: '04', title: '区块链锚定', desc: '将你的 GyID 锚定到 Polygon 或 Aptos 区块链。', cta: '查看文档 →', href: '#' },
        { n: '05', title: 'H3 网格 API', desc: '使用地理六边形和空间身份。', cta: '查看文档 →', href: '#' },
        { n: '06', title: 'NFT 开发', desc: '使用 GyID 构建位置锚定的 NFT 应用。', cta: '查看文档 →', href: '#' },
      ],
      wip: {
        badge: '文档建设中',
        desc: '完整文档正在编写中，请关注 GitHub 获取最新进展。',
        btn: { label: 'GitHub Repository ↗', href: 'https://github.com/huanghe2026/GYID', external: true },
      },
    },

    notFound: {
      code: '404',
      title: '页面未找到',
      desc: '这个位置不在我们的地图上',
      btn: { label: '返回首页', href: '/zh' },
    },
  },
};

export type Locale = 'en' | 'zh';
