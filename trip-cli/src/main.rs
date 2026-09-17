//! TRIP 协议 CLI — 面包屑、epoch、PoH 证书的命令行工具。
//!
//! 数据格式：CBOR 结构以 hex 编码，链文件每行一条 hex CBOR。
//!
//! 运行 `trip --help` 查看完整命令列表。

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use trip_core::breadcrumb::{Breadcrumb, MetaFlags};
use trip_core::chain::ChainRules;
use trip_core::crypto::ProtocolKey;
use trip_core::engine::behavior::{BehavioralProfile, BreadcrumbView};
use trip_core::engine::hamiltonian::evaluate;
use trip_core::engine::levy::fit as levy_fit;
use trip_core::engine::psd::{displacements_from_cells, psd_alpha};
use trip_core::engine::sim::{trip_walk_path, SimConfig, TripConfig};
use trip_core::engine::trust::{alpha_in_bio_range, can_claim_handle, trust_score, TrustInput};
use trip_core::epoch::Epoch;
use trip_core::poh::PohCertificate;

#[derive(Parser)]
#[command(name = "trip", version, about = "TRIP protocol CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 生成新 Ed25519 密钥对（输出 seed + pubkey hex）
    Keygen,
    /// 面包屑签名 / 验签
    Breadcrumb {
        #[command(subcommand)]
        action: BreadcrumbCmd,
    },
    /// 验证面包屑链（去重、间隔、签名、哈希链）
    Chain {
        /// 链文件路径（每行一条 hex CBOR 面包屑）
        file: PathBuf,
    },
    /// Epoch 封装 / 验证
    Epoch {
        #[command(subcommand)]
        action: EpochCmd,
    },
    /// PoH 证书签发 / 验签
    Poh {
        #[command(subcommand)]
        action: PohCmd,
    },
    /// 端到端仿真（生成轨迹 → 面包屑 → 画像 → PoH）
    Simulate {
        /// 随机种子
        #[arg(long, default_value = "2024")]
        seed: u64,
        /// 面包屑数量
        #[arg(long, default_value = "512")]
        count: usize,
    },
}

#[derive(Subcommand)]
enum BreadcrumbCmd {
    /// 签名一条面包屑
    Sign {
        /// 身份密钥 seed（64 hex chars）
        #[arg(long)]
        seed: String,
        /// 面包屑序号
        #[arg(long)]
        index: u64,
        /// Unix 秒时间戳
        #[arg(long)]
        timestamp: u64,
        /// H3 cell index（u64）
        #[arg(long)]
        cell: u64,
        /// H3 分辨率
        #[arg(long, default_value = "10")]
        resolution: u8,
        /// 前一块哈希（64 hex chars，创世省略）
        #[arg(long)]
        prev: Option<String>,
        /// 探索会话标志（允许较短间隔）
        #[arg(long)]
        exploration: bool,
    },
    /// 验证一条面包屑的签名
    Verify {
        /// hex CBOR 面包屑
        hex_cbor: String,
    },
}

#[derive(Subcommand)]
enum EpochCmd {
    /// 从面包屑链封装 epoch（从 stdin 读 hex CBOR，每行一条）
    Seal {
        /// 身份密钥 seed（64 hex chars）
        #[arg(long)]
        seed: String,
        /// epoch 序号
        #[arg(long)]
        number: u64,
    },
    /// 验证 epoch 签名 + Merkle 覆盖
    Verify {
        /// hex CBOR epoch
        hex_cbor: String,
        /// 链文件路径
        file: PathBuf,
    },
}

#[derive(Subcommand)]
enum PohCmd {
    /// 签发 PoH 证书（Verifier 侧）
    Issue {
        /// Verifier 密钥 seed（64 hex chars）
        #[arg(long)]
        verifier_seed: String,
        /// 被证明身份公钥（64 hex chars）
        #[arg(long)]
        identity: String,
        /// PSD α
        #[arg(long)]
        alpha: f64,
        /// Levy β
        #[arg(long)]
        beta: f64,
        /// Levy κ
        #[arg(long)]
        kappa: f64,
        /// 临界置信度
        #[arg(long)]
        confidence: f64,
        /// 信任分
        #[arg(long)]
        trust: f64,
        /// unique cell 数
        #[arg(long)]
        unique_cells: u64,
        /// 面包屑总数
        #[arg(long)]
        breadcrumb_count: u64,
        /// RP nonce（32 hex chars）
        #[arg(long, default_value = "000102030405060708090a0b0c0d0e0f")]
        nonce: String,
        /// 链头哈希（64 hex chars，默认全零）
        #[arg(
            long,
            default_value = "0000000000000000000000000000000000000000000000000000000000000000"
        )]
        chain_head: String,
    },
    /// 验证 PoH 证书（RP 侧）
    Verify {
        /// hex CBOR PoH 证书
        hex_cbor: String,
        /// Verifier 公钥（64 hex chars）
        #[arg(long)]
        verifier_pubkey: String,
        /// RP nonce（32 hex chars）
        #[arg(long)]
        nonce: String,
        /// 当前 Unix 秒
        #[arg(long)]
        now: u64,
        /// 最低置信度
        #[arg(long, default_value = "0.1")]
        min_confidence: f64,
        /// 最低信任分
        #[arg(long, default_value = "20.0")]
        min_trust: f64,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Keygen => cmd_keygen(),
        Cmd::Breadcrumb { action } => match action {
            BreadcrumbCmd::Sign {
                seed,
                index,
                timestamp,
                cell,
                resolution,
                prev,
                exploration,
            } => cmd_breadcrumb_sign(&seed, index, timestamp, cell, resolution, prev, exploration),
            BreadcrumbCmd::Verify { hex_cbor } => cmd_breadcrumb_verify(&hex_cbor),
        },
        Cmd::Chain { file } => cmd_chain_verify(&file),
        Cmd::Epoch { action } => match action {
            EpochCmd::Seal { seed, number } => cmd_epoch_seal(&seed, number),
            EpochCmd::Verify { hex_cbor, file } => cmd_epoch_verify(&hex_cbor, &file),
        },
        Cmd::Poh { action } => match action {
            PohCmd::Issue {
                verifier_seed,
                identity,
                alpha,
                beta,
                kappa,
                confidence,
                trust,
                unique_cells,
                breadcrumb_count,
                nonce,
                chain_head,
            } => cmd_poh_issue(
                &verifier_seed,
                &identity,
                alpha,
                beta,
                kappa,
                confidence,
                trust,
                unique_cells,
                breadcrumb_count,
                &nonce,
                &chain_head,
            ),
            PohCmd::Verify {
                hex_cbor,
                verifier_pubkey,
                nonce,
                now,
                min_confidence,
                min_trust,
            } => cmd_poh_verify(
                &hex_cbor,
                &verifier_pubkey,
                &nonce,
                now,
                min_confidence,
                min_trust,
            ),
        },
        Cmd::Simulate { seed, count } => cmd_simulate(seed, count),
    }
}

// ── 工具函数 ──

fn parse_seed(hex_str: &str) -> [u8; 32] {
    hex::decode(hex_str)
        .ok()
        .and_then(|v| v.try_into().ok())
        .unwrap_or_else(|| {
            eprintln!("错误：seed 必须是 64 个 hex 字符");
            std::process::exit(1);
        })
}

fn parse_32bytes(hex_str: &str, name: &str) -> [u8; 32] {
    hex::decode(hex_str)
        .ok()
        .and_then(|v| v.try_into().ok())
        .unwrap_or_else(|| {
            eprintln!("错误：{name} 必须是 64 个 hex 字符");
            std::process::exit(1);
        })
}

fn parse_16bytes(hex_str: &str, name: &str) -> [u8; 16] {
    hex::decode(hex_str)
        .ok()
        .and_then(|v| v.try_into().ok())
        .unwrap_or_else(|| {
            eprintln!("错误：{name} 必须是 32 个 hex 字符");
            std::process::exit(1);
        })
}

fn read_chain_from_stdin() -> Vec<Breadcrumb> {
    let stdin = io::stdin();
    let mut crumbs = Vec::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        let line = line.unwrap_or_else(|e| {
            eprintln!("stdin 读取错误（行 {}）：{e}", i + 1);
            std::process::exit(1);
        });
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let bytes = hex::decode(trimmed).unwrap_or_else(|e| {
            eprintln!("hex 解码错误（行 {}）：{e}", i + 1);
            std::process::exit(1);
        });
        let bc = Breadcrumb::from_cbor(&bytes).unwrap_or_else(|e| {
            eprintln!("CBOR 解析错误（行 {}）：{e}", i + 1);
            std::process::exit(1);
        });
        crumbs.push(bc);
    }
    crumbs
}

fn read_chain_from_file(path: &std::path::Path) -> Vec<Breadcrumb> {
    let content = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("无法读取文件 {}：{e}", path.display());
        std::process::exit(1);
    });
    let mut crumbs = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let bytes = hex::decode(trimmed).unwrap_or_else(|e| {
            eprintln!("hex 解码错误（行 {}）：{e}", i + 1);
            std::process::exit(1);
        });
        let bc = Breadcrumb::from_cbor(&bytes).unwrap_or_else(|e| {
            eprintln!("CBOR 解析错误（行 {}）：{e}", i + 1);
            std::process::exit(1);
        });
        crumbs.push(bc);
    }
    crumbs
}

// ── 命令实现 ──

fn cmd_keygen() {
    use rand::rngs::OsRng;
    use rand::RngCore;
    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);
    let key = ProtocolKey::from_seed(&seed);
    let pubkey = hex::encode(key.public_bytes());
    println!("seed     = {}", hex::encode(seed));
    println!("pubkey   = {pubkey}");
    eprintln!("# 保管 seed，它是你的身份私钥。pubkey 可公开。");
}

fn cmd_breadcrumb_sign(
    seed_hex: &str,
    index: u64,
    timestamp: u64,
    cell: u64,
    resolution: u8,
    prev_hex: Option<String>,
    exploration: bool,
) {
    let seed = parse_seed(seed_hex);
    let key = ProtocolKey::from_seed(&seed);
    let prev = prev_hex.map(|h| parse_32bytes(&h, "prev"));
    let mut flags = MetaFlags::new();
    flags.exploration = exploration;
    let mut bc = Breadcrumb::new_unsigned(
        index,
        key.public_bytes(),
        timestamp,
        cell,
        resolution,
        [0u8; 32],
        prev,
        flags,
    );
    bc.sign(&key).unwrap_or_else(|e| {
        eprintln!("签名失败：{e}");
        std::process::exit(1);
    });
    let cbor = bc.to_cbor();
    let block_hash = bc.block_hash();
    println!("{}", hex::encode(&cbor));
    eprintln!("pubkey    = {}", hex::encode(bc.identity));
    eprintln!("block_hash = {}", hex::encode(block_hash));
}

fn cmd_breadcrumb_verify(hex_cbor: &str) {
    let bytes = hex::decode(hex_cbor).unwrap_or_else(|e| {
        eprintln!("hex 解码错误：{e}");
        std::process::exit(1);
    });
    let bc = Breadcrumb::from_cbor(&bytes).unwrap_or_else(|e| {
        eprintln!("CBOR 解析错误：{e}");
        std::process::exit(1);
    });
    match bc.verify_signature() {
        Ok(()) => {
            println!(
                "OK  index={}  cell={}  ts={}  res={}",
                bc.index, bc.h3_cell, bc.timestamp, bc.h3_resolution
            );
        }
        Err(e) => {
            println!("FAIL  {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_chain_verify(file: &std::path::Path) {
    let crumbs = read_chain_from_file(file);
    if crumbs.is_empty() {
        eprintln!("链文件为空");
        std::process::exit(1);
    }
    print!("验证 {} 条面包屑……", crumbs.len());
    io::stdout().flush().ok();
    match ChainRules::default().verify(&crumbs) {
        Ok(()) => {
            println!(" OK");
            println!("index 范围: 0..={}", crumbs.len() - 1);
            println!("身份公钥: {}", hex::encode(crumbs[0].identity));
            let chain_head = crumbs.last().unwrap().block_hash();
            println!("链头哈希: {}", hex::encode(chain_head));
        }
        Err(e) => {
            println!(" FAIL");
            eprintln!("错误：{e}");
            std::process::exit(1);
        }
    }
}

fn cmd_epoch_seal(seed_hex: &str, number: u64) {
    let seed = parse_seed(seed_hex);
    let key = ProtocolKey::from_seed(&seed);
    let crumbs = read_chain_from_stdin();
    if crumbs.is_empty() {
        eprintln!("stdin 无面包屑");
        std::process::exit(1);
    }
    let epoch = Epoch::seal(number, &crumbs, &key).unwrap_or_else(|e| {
        eprintln!("封装失败：{e}");
        std::process::exit(1);
    });
    println!("{}", hex::encode(epoch.to_cbor()));
    eprintln!(
        "epoch {}  index {}..={}  unique_cells={}  merkle={}",
        epoch.number,
        epoch.first_index,
        epoch.last_index,
        epoch.unique_cells,
        hex::encode(epoch.merkle_root)
    );
}

fn cmd_epoch_verify(hex_cbor: &str, file: &std::path::Path) {
    let bytes = hex::decode(hex_cbor).unwrap_or_else(|e| {
        eprintln!("hex 解码错误：{e}");
        std::process::exit(1);
    });
    let epoch = Epoch::from_cbor(&bytes).unwrap_or_else(|e| {
        eprintln!("CBOR 解析错误：{e}");
        std::process::exit(1);
    });
    let crumbs = read_chain_from_file(file);
    print!("验证 epoch {} 签名……", epoch.number);
    io::stdout().flush().ok();
    epoch.verify_signature().unwrap_or_else(|e| {
        println!(" FAIL");
        eprintln!("签名验证失败：{e}");
        std::process::exit(1);
    });
    print!(" OK\n验证 Merkle 覆盖……");
    io::stdout().flush().ok();
    match epoch.verify_coverage(&crumbs) {
        Ok(()) => println!(" OK"),
        Err(e) => {
            println!(" FAIL");
            eprintln!("覆盖验证失败：{e}");
            std::process::exit(1);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_poh_issue(
    verifier_seed_hex: &str,
    identity_hex: &str,
    alpha: f64,
    beta: f64,
    kappa: f64,
    confidence: f64,
    trust: f64,
    unique_cells: u64,
    breadcrumb_count: u64,
    nonce_hex: &str,
    chain_head_hex: &str,
) {
    let verifier_seed = parse_seed(verifier_seed_hex);
    let verifier = ProtocolKey::from_seed(&verifier_seed);
    let identity = parse_32bytes(identity_hex, "identity");
    let nonce = parse_16bytes(nonce_hex, "nonce");
    let chain_head = parse_32bytes(chain_head_hex, "chain_head");
    let issued_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let cert = PohCertificate::issue(
        &verifier,
        identity,
        issued_at,
        1,
        alpha,
        beta,
        kappa,
        0.85,
        confidence,
        trust,
        unique_cells,
        breadcrumb_count,
        3600,
        nonce,
        chain_head,
    );
    println!("{}", hex::encode(cert.to_cbor()));
    eprintln!("PoH 证书已签发（{} 字节 CBOR）", cert.to_cbor().len());
}

fn cmd_poh_verify(
    hex_cbor: &str,
    verifier_pubkey_hex: &str,
    nonce_hex: &str,
    now: u64,
    min_confidence: f64,
    min_trust: f64,
) {
    let bytes = hex::decode(hex_cbor).unwrap_or_else(|e| {
        eprintln!("hex 解码错误：{e}");
        std::process::exit(1);
    });
    let cert = PohCertificate::from_cbor(&bytes).unwrap_or_else(|e| {
        eprintln!("CBOR 解析错误：{e}");
        std::process::exit(1);
    });
    let verifier_pubkey = parse_32bytes(verifier_pubkey_hex, "verifier_pubkey");
    let nonce = parse_16bytes(nonce_hex, "nonce");
    print!("验签 + 新鲜性……");
    io::stdout().flush().ok();
    cert.verify_freshness(&verifier_pubkey, &nonce, now)
        .unwrap_or_else(|e| {
            println!(" FAIL");
            eprintln!("{e}");
            std::process::exit(1);
        });
    print!(" OK\n策略检查……");
    io::stdout().flush().ok();
    let policy_ok = cert.meets_policy(min_confidence, min_trust);
    println!(" {}", if policy_ok { "PASS" } else { "FAIL" });
    println!(
        "α={:.4}  confidence={:.4}  trust={:.2}  unique={}  crumbs={}",
        cert.alpha,
        cert.criticality_confidence,
        cert.trust,
        cert.unique_cells,
        cert.breadcrumb_count
    );
    if !policy_ok {
        std::process::exit(1);
    }
}

fn cmd_simulate(seed: u64, count: usize) {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let mut rng = StdRng::seed_from_u64(seed);
    let identity_key = ProtocolKey::from_seed(&[42u8; 32]);
    let verifier_key = ProtocolKey::from_seed(&[43u8; 32]);

    /* Attester：生成轨迹 + 面包屑链 */
    let cfg = SimConfig {
        raw_step_cap: 20_000,
        ..SimConfig::default()
    };
    let trip = TripConfig::default();
    let path = trip_walk_path(&mut rng, 1.75, 5.0, count, &cfg, &trip);
    let id_pub = identity_key.public_bytes();
    let mut crumbs = Vec::with_capacity(path.cells.len());
    let mut prev_hash: Option<[u8; 32]> = None;
    for (i, &cell) in path.cells.iter().enumerate() {
        let mut bc = Breadcrumb::new_unsigned(
            i as u64,
            id_pub,
            1700000000 + (i as u64) * 900,
            cell,
            10,
            [0u8; 32],
            prev_hash,
            MetaFlags::new(),
        );
        bc.sign(&identity_key).expect("sign");
        let hash = bc.block_hash();
        crumbs.push((bc, hash));
        prev_hash = Some(hash);
    }

    /* Verifier：链验证 + 画像 + 评估 */
    let breadcrumb_chain: Vec<Breadcrumb> = crumbs.iter().map(|(bc, _)| bc.clone()).collect();
    ChainRules::default()
        .verify(&breadcrumb_chain)
        .expect("chain");
    let views: Vec<BreadcrumbView> = crumbs
        .iter()
        .map(|(bc, hash)| BreadcrumbView {
            ts: bc.timestamp as i64,
            cell: bc.h3_cell,
            prev_hash: bc.prev_hash,
            block_hash: *hash,
            imu_present: false,
        })
        .collect();
    let profile = BehavioralProfile::from_breadcrumbs(&views).expect("profile");
    let disp = displacements_from_cells(&path.cells).expect("displacements");
    let psd = psd_alpha(&disp).expect("psd");
    let levy = levy_fit(&disp, None).expect("levy");
    let last = views.last().unwrap();
    let prev = &views[views.len() - 2];
    let last_disp = disp.last().copied().unwrap_or(0.0);
    let h = evaluate(
        &profile,
        last,
        last_disp,
        &[],
        Some((0.1, 0.05)),
        prev.ts,
        prev.cell,
        true,
    );
    let trust_input = TrustInput {
        breadcrumb_count: crumbs.len(),
        unique_cells: profile.unique_cells,
        days_since_first: 1.0,
        chain_integrity: true,
    };
    let alpha_ok = alpha_in_bio_range(psd.alpha);
    let t = trust_score(&trust_input, alpha_ok);

    println!("=== TRIP 仿真（seed={seed}, crumbs={count}）===");
    println!(
        "面包屑链:  {} 条,  unique cells: {}",
        crumbs.len(),
        profile.unique_cells
    );
    println!(
        "PSD:       α = {:.4}  (R² = {:.4}, class = {})",
        psd.alpha,
        psd.r_squared,
        psd.classification.as_str()
    );
    println!("Levy MLE:  β = {:.4}  κ = {:.4}", levy.beta, levy.kappa);
    println!(
        "Hamiltonian: H = {:.4}  (alert = {}, baseline = {:.4})",
        h.total,
        h.alert.as_str(),
        h.baseline
    );
    println!("  spatial={:.3}  temporal={:.3}  kinetic={:.3}  flock={:.3}  contextual={:.3}  structure={:.3}",
        h.spatial, h.temporal, h.kinetic, h.flock, h.contextual, h.structure);
    println!(
        "Trust:     T = {:.2}  (α∈bio = {}, handle_eligible = {})",
        t,
        alpha_ok,
        can_claim_handle(&trust_input, alpha_ok)
    );

    /* PoH 签发 */
    let chain_head = crumbs.last().unwrap().1;
    let nonce = [0xAAu8; 16];
    let cert = PohCertificate::issue(
        &verifier_key,
        id_pub,
        1700256000,
        1,
        psd.alpha,
        levy.beta,
        levy.kappa,
        0.85,
        psd.confidence,
        t,
        profile.unique_cells as u64,
        crumbs.len() as u64,
        3600,
        nonce,
        chain_head,
    );
    let cbor = cert.to_cbor();
    println!("PoH:       {} 字节 CBOR", cbor.len());

    /* RP 验签 */
    let parsed = PohCertificate::from_cbor(&cbor).expect("parse");
    parsed
        .verify_freshness(&verifier_key.public_bytes(), &nonce, 1700256001)
        .expect("freshness");
    let policy_ok = parsed.meets_policy(0.1, 20.0);
    println!(
        "RP 验签:   signature OK, confidence = {:.4}, policy = {}",
        parsed.criticality_confidence,
        if policy_ok { "PASS" } else { "FAIL" }
    );
    println!("\n=== {} ===", if policy_ok { "PASS" } else { "FAIL" });
}

// ── ProtocolKey 扩展（仅 CLI 用）──

// trip-core 有意不暴露私钥序列化（安全设计）。
// keygen 命令直接生成 32 字节 seed 并打印，再用 from_seed 构造密钥获取公钥。
