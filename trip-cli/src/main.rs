//! GyID CLI — 身份管理、面包屑采集、Active Verification、PoH 证书。
//!
//! 用户命令（面向终端用户）：
//!   gyid init / collect / verify / poh-list / poh-show
//!
//! 高级命令（协议开发者，advanced）：
//!   gyid keygen / breadcrumb / chain / epoch / poh / simulate
//!
//! 运行 `gyid --help` 查看完整命令列表。

mod anchor_cmds;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use anchor_cmds::AnchorCmd;
use clap::{Parser, Subcommand};
use trip_core::breadcrumb::{Breadcrumb, MetaFlags};
use trip_core::chain::ChainRules;
use trip_core::crypto::ProtocolKey;
use trip_core::did::{AnchorReference, DidDocument, DidDocumentConfig};
use trip_core::engine::behavior::{BehavioralProfile, BreadcrumbView};
use trip_core::engine::hamiltonian::evaluate;
use trip_core::engine::levy::fit as levy_fit;
use trip_core::engine::psd::{displacements_from_cells, psd_alpha};
use trip_core::engine::sim::{trip_walk_path, SimConfig, TripConfig};
use trip_core::engine::trust::{alpha_in_bio_range, can_claim_handle, trust_score, TrustInput};
use trip_core::epoch::Epoch;
use trip_core::poh::PohCertificate;
use trip_core::tit::{Tit, TitClaims, TitIssuer};

#[derive(Parser)]
#[command(name = "gyid", version, about = "GyID CLI — 身份 / 采集 / 验证 / PoH")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    // ── 用户命令 ──
    /// 生成新 GyID 身份（输出 seed + pubkey）
    Init,

    /// 持续采集面包屑并上传到 Verifier（daemon 模式，需 GPS）
    Collect {
        /// Verifier URL（如 http://localhost:8080）
        #[arg(long)]
        verifier: String,
        /// 身份 seed（64 hex chars）
        #[arg(long)]
        seed: String,
        /// 采集间隔（秒，≥900 符合 ChainRules）
        #[arg(long, default_value = "900")]
        interval: u64,
    },

    /// 发起 Active Verification（RP 角色，轮询取回 PoH）
    Verify {
        /// Verifier URL
        #[arg(long)]
        verifier: String,
        /// 被验证方公钥（64 hex chars）
        #[arg(long)]
        attester: String,
        /// RP nonce（32 hex chars，不传则随机生成）
        #[arg(long)]
        rp_nonce: Option<String>,
    },

    /// 列出某 attester 已签发的 PoH challenge_id
    PohList {
        /// Verifier URL
        #[arg(long)]
        verifier: String,
        /// 被验证方公钥（64 hex chars）
        #[arg(long)]
        attester: String,
    },

    /// 解析并展示 PoH 证书字段（pretty-print）
    PohShow {
        /// PoH 证书 hex CBOR
        hex_cbor: String,
    },

    // ── 高级命令（协议开发者）──
    /// (advanced) 生成 Ed25519 密钥对
    Keygen,
    /// (advanced) 面包屑签名 / 验签
    Breadcrumb {
        #[command(subcommand)]
        action: BreadcrumbCmd,
    },
    /// (advanced) 验证面包屑链
    Chain {
        /// 链文件路径（每行一条 hex CBOR 面包屑）
        file: PathBuf,
    },
    /// (advanced) Epoch 封装 / 验证
    Epoch {
        #[command(subcommand)]
        action: EpochCmd,
    },
    /// (advanced) PoH 证书签发 / 验签
    Poh {
        #[command(subcommand)]
        action: PohCmd,
    },
    /// (advanced) did:geoyuan 生成 / 解析 / DID Document
    Did {
        #[command(subcommand)]
        action: DidCmd,
    },
    /// (advanced) TIT（轨迹身份令牌）签发 / 验签
    Tit {
        #[command(subcommand)]
        action: TitCmd,
    },
    /// (advanced) EVM 链上锚定（GeoTITRegistry）
    Anchor {
        #[command(subcommand)]
        action: AnchorCmd,
    },
    /// (advanced) 端到端仿真
    Simulate {
        #[arg(long, default_value = "2024")]
        seed: u64,
        #[arg(long, default_value = "512")]
        count: usize,
    },
}

#[derive(Subcommand)]
enum BreadcrumbCmd {
    /// 签名一条面包屑
    Sign {
        #[arg(long)]
        seed: String,
        #[arg(long)]
        index: u64,
        #[arg(long)]
        timestamp: u64,
        #[arg(long)]
        cell: u64,
        #[arg(long, default_value = "10")]
        resolution: u8,
        #[arg(long)]
        prev: Option<String>,
        #[arg(long)]
        exploration: bool,
    },
    /// 验证一条面包屑的签名
    Verify { hex_cbor: String },
}

#[derive(Subcommand)]
enum EpochCmd {
    /// 从面包屑链封装 epoch（从 stdin 读 hex CBOR，每行一条）
    Seal {
        #[arg(long)]
        seed: String,
        #[arg(long)]
        number: u64,
    },
    /// 验证 epoch 签名 + Merkle 覆盖
    Verify { hex_cbor: String, file: PathBuf },
}

#[derive(Subcommand)]
enum PohCmd {
    /// 签发 PoH 证书（Verifier 侧）
    Issue {
        #[arg(long)]
        verifier_seed: String,
        #[arg(long)]
        identity: String,
        #[arg(long)]
        alpha: f64,
        #[arg(long)]
        beta: f64,
        #[arg(long)]
        kappa: f64,
        #[arg(long)]
        confidence: f64,
        #[arg(long)]
        trust: f64,
        #[arg(long)]
        unique_cells: u64,
        #[arg(long)]
        breadcrumb_count: u64,
        #[arg(long, default_value = "000102030405060708090a0b0c0d0e0f")]
        nonce: String,
        #[arg(
            long,
            default_value = "0000000000000000000000000000000000000000000000000000000000000000"
        )]
        chain_head: String,
    },
    /// 验证 PoH 证书（RP 侧）
    Verify {
        hex_cbor: String,
        #[arg(long)]
        verifier_pubkey: String,
        #[arg(long)]
        nonce: String,
        #[arg(long)]
        now: u64,
        #[arg(long, default_value = "0.1")]
        min_confidence: f64,
        #[arg(long, default_value = "20.0")]
        min_trust: f64,
    },
}

#[derive(Subcommand)]
enum DidCmd {
    /// 从身份 seed 生成 DID + W3C DID Document（JSON）
    Show {
        /// 身份 seed（64 hex chars）
        #[arg(long)]
        seed: String,
        /// Verifier base URL（写入 #verifier 与 #tit 端点）
        #[arg(long)]
        verifier: Option<String>,
        /// GPv1 libp2p multiaddr（#p2p 端点）
        #[arg(long)]
        p2p: Option<String>,
        /// geoyuan.com 展示名（@nickname）
        #[arg(long)]
        handle: Option<String>,
        /// EVM 锚定指针，形如 eip155:8453:0xRegistry
        #[arg(long)]
        anchor: Option<String>,
    },
    /// 解析 DID 回 Ed25519 公钥
    Resolve {
        /// did:geoyuan:z…
        did: String,
    },
}

#[derive(Subcommand)]
enum TitCmd {
    /// 签发 TIT（默认身份自签；--issuer verifier 用 Verifier 密钥背书）
    Issue {
        /// 签发者 seed（64 hex chars）
        #[arg(long)]
        seed: String,
        /// --issuer verifier 时的被证明方公钥（64 hex chars）
        #[arg(long)]
        identity: Option<String>,
        /// 签发者：identity | verifier
        #[arg(long, default_value = "identity")]
        issuer: String,
        #[arg(long)]
        epochs: u64,
        #[arg(long)]
        breadcrumbs: u64,
        #[arg(long)]
        unique_cells: u64,
        #[arg(long)]
        trust: f64,
        /// 有效期（秒）
        #[arg(long, default_value = "3600")]
        validity: u64,
        /// 签发时间（Unix 秒；默认当前时间）
        #[arg(long)]
        issued_at: Option<u64>,
    },
    /// 验签并展示 TIT（Base64url 或 hex CBOR）
    Verify {
        tit: String,
        /// verifier 签发时的 Verifier 公钥（64 hex chars）
        #[arg(long)]
        verifier_pubkey: Option<String>,
        /// 当前时间（Unix 秒；默认当前时间）
        #[arg(long)]
        now: Option<u64>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        // ── 用户命令 ──
        Cmd::Init => cmd_init(),
        Cmd::Collect {
            verifier,
            seed,
            interval,
        } => cmd_collect(&verifier, &seed, interval).await,
        Cmd::Verify {
            verifier,
            attester,
            rp_nonce,
        } => cmd_verify(&verifier, &attester, rp_nonce).await,
        Cmd::PohList { verifier, attester } => cmd_poh_list(&verifier, &attester).await,
        Cmd::PohShow { hex_cbor } => cmd_poh_show(&hex_cbor),

        // ── 高级命令 ──
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
        Cmd::Did { action } => match action {
            DidCmd::Show {
                seed,
                verifier,
                p2p,
                handle,
                anchor,
            } => cmd_did_show(
                &seed,
                verifier.as_deref(),
                p2p.as_deref(),
                handle.as_deref(),
                anchor.as_deref(),
            ),
            DidCmd::Resolve { did } => cmd_did_resolve(&did),
        },
        Cmd::Tit { action } => match action {
            TitCmd::Issue {
                seed,
                identity,
                issuer,
                epochs,
                breadcrumbs,
                unique_cells,
                trust,
                validity,
                issued_at,
            } => cmd_tit_issue(
                &seed,
                identity.as_deref(),
                &issuer,
                epochs,
                breadcrumbs,
                unique_cells,
                trust,
                validity,
                issued_at,
            ),
            TitCmd::Verify {
                tit,
                verifier_pubkey,
                now,
            } => cmd_tit_verify(&tit, verifier_pubkey.as_deref(), now),
        },
        Cmd::Anchor { action } => anchor_cmds::dispatch(action).await,
        Cmd::Simulate { seed, count } => cmd_simulate(seed, count),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 用户命令实现
// ═══════════════════════════════════════════════════════════════════════

/// `gyid init`：生成新 GyID 身份。
fn cmd_init() {
    let id = gyid_shared::Identity::generate();
    println!("seed    = {}", id.seed_hex());
    println!("pubkey  = {}", id.pubkey_hex());
    eprintln!("✓ 身份已创建，请保管好 seed");
}

/// `gyid collect`：daemon 模式采集面包屑并上传。
async fn cmd_collect(verifier_url: &str, seed_hex: &str, interval: u64) {
    let identity = match gyid_shared::Identity::from_seed_hex(seed_hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("✗ seed 无效：{e}");
            std::process::exit(1);
        }
    };
    let pubkey_hex = identity.pubkey_hex();
    let client = gyid_shared::VerifierClient::new(verifier_url);

    // 拉取当前链状态（index / chain_head / last_ts）
    let identity_info = match client.get_identity(&pubkey_hex).await {
        Ok(info) => info,
        Err(_) => {
            eprintln!("（链不存在，将从 index=0 开始）");
            gyid_shared::verifier_client::IdentityInfo {
                attester: pubkey_hex.clone(),
                breadcrumb_count: 0,
                unique_cells: 0,
                chain_head: String::new(),
                last_ts: 0,
            }
        }
    };

    let mut index = identity_info.breadcrumb_count;
    let mut prev_hash: Option<[u8; 32]> = if identity_info.chain_head.is_empty() {
        None
    } else {
        Some(parse_32bytes(&identity_info.chain_head, "chain_head"))
    };
    let mut last_ts = identity_info.last_ts;

    eprintln!("身份: {pubkey_hex}");
    eprintln!(
        "当前链: {} 条, 链头: {}",
        identity_info.breadcrumb_count,
        {
            let h = &identity_info.chain_head;
            if h.is_empty() {
                "(空)".into()
            } else {
                h[..16].to_string()
            }
        }
    );
    eprintln!("采集间隔: {interval}s  (Ctrl-C 停止)");

    // 先探测定位源：无 GPS 时快速失败，避免空等一个 interval
    let first_pos = match get_location() {
        Some(pos) => pos,
        None => {
            eprintln!("✗ 无法获取 GPS 位置（未找到定位源）");
            eprintln!(
                "  Linux 需 geoclue2 / GPS 硬件；开发测试可用 gyid-shared 的 seed_chain 夹具"
            );
            std::process::exit(1);
        }
    };
    let mut pending_pos: Option<(f64, f64)> = Some(first_pos);

    loop {
        // 等待间隔（首轮已探到位置则立即采集）
        if pending_pos.is_none() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let wait = if last_ts == 0 {
                interval
            } else {
                interval.saturating_sub(now.saturating_sub(last_ts))
            };
            if wait > 0 {
                eprintln!("等待 {wait}s 到下一个采集点…");
                tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
            }
        }

        // 获取 GPS 位置
        let (lat, lng) = match pending_pos.take().or_else(get_location) {
            Some(pos) => pos,
            None => {
                eprintln!("✗ 无法获取 GPS 位置（定位源丢失）");
                std::process::exit(1);
            }
        };

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // 采集面包屑
        let bc = match gyid_shared::collect_breadcrumb(
            &identity, lat, lng, 10, ts, index, prev_hash, false, None,
        ) {
            Ok(bc) => bc,
            Err(e) => {
                eprintln!("✗ 采集失败：{e}");
                continue;
            }
        };
        let cbor = bc.to_cbor();

        eprintln!(
            "采集 #{}: lat={:.5} lng={:.5} cell={} ts={}",
            index, lat, lng, bc.h3_cell, ts
        );

        // 上传
        match client.upload_evidence(cbor).await {
            Ok(resp) => {
                eprintln!(
                    "  上传成功: stored={} unique={} head={}",
                    resp.stored,
                    resp.unique_cells,
                    &resp.chain_head[..16]
                );
                index = resp.stored;
                prev_hash = Some(parse_32bytes(&resp.chain_head, "chain_head"));
                last_ts = ts;
            }
            Err(e) => {
                eprintln!("  上传失败：{e}");
            }
        }
    }
}

/// `gyid verify`：RP 发起 Active Verification，轮询取回 PoH。
async fn cmd_verify(verifier_url: &str, attester_hex: &str, rp_nonce_opt: Option<String>) {
    let client = gyid_shared::VerifierClient::new(verifier_url);

    // 生成或使用传入的 rp_nonce
    let rp_nonce_hex = match rp_nonce_opt {
        Some(n) => n,
        None => {
            use rand::RngCore;
            let mut buf = [0u8; 16];
            rand::rngs::OsRng.fill_bytes(&mut buf);
            hex::encode(buf)
        }
    };

    eprintln!("RP nonce: {rp_nonce_hex}");
    eprintln!("Attester: {attester_hex}");

    // 1) 请求挑战
    eprintln!("POST /v1/verify …");
    let challenge = match client.request_challenge(attester_hex, &rp_nonce_hex).await {
        Ok(info) => info,
        Err(e) => {
            eprintln!("✗ 请求挑战失败：{e}");
            std::process::exit(1);
        }
    };
    eprintln!(
        "挑战 {} 已创建，过期 {}，delivered={}",
        &challenge.challenge_id[..8.min(challenge.challenge_id.len())],
        challenge.expires_at,
        challenge.delivered
    );
    if !challenge.delivered {
        eprintln!("✗ 挑战未被推送到 Attester（delivered=false），请确认 Attester 已连 WS");
        std::process::exit(1);
    }

    // 2) 提示用户在 Attester 端签名
    eprintln!("\n请在 Attester 端完成签名响应（CLI 不能自动签名）。");
    eprintln!("等待 PoH 签发…");

    // 3) 轮询 PoH
    let poll_deadline = challenge.expires_at + 60; // 额外 60s 缓冲
    let poh_bytes: Vec<u8>;
    loop {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if now > poll_deadline {
            eprintln!("✗ 轮询超时");
            std::process::exit(1);
        }

        match client.fetch_poh(&challenge.challenge_id).await {
            Ok(gyid_shared::verifier_client::PohFetch::Issued(bytes)) => {
                poh_bytes = bytes;
                break;
            }
            Ok(gyid_shared::verifier_client::PohFetch::Pending { expires_at }) => {
                eprintln!("  Pending (expires_at={expires_at})，1s 后重试…");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            Err(e) => {
                eprintln!("  轮询错误：{e}，2s 后重试…");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }

    let poh_hex = hex::encode(&poh_bytes);
    eprintln!("\nPoH 证书已取回（{} 字节）", poh_bytes.len());
    println!("{poh_hex}");

    // 4) 本地校验
    let vk_hex = match client.fetch_verifier_pubkey_hex().await {
        Ok(vk) => vk,
        Err(e) => {
            eprintln!("✗ 获取 Verifier 公钥失败：{e}");
            std::process::exit(1);
        }
    };
    let vk = parse_32bytes(&vk_hex, "verifier_pubkey");
    let nonce = parse_16bytes(&rp_nonce_hex, "rp_nonce");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let info =
        gyid_shared::verify_poh(&poh_bytes, &vk, &nonce, now, 0.1, 20.0).expect("verify_poh");

    eprintln!();
    print_poh_info(&info);
}

/// `gyid poh-list`：列出某 attester 已签发的 PoH。
async fn cmd_poh_list(verifier_url: &str, attester_hex: &str) {
    let client = gyid_shared::VerifierClient::new(verifier_url);

    match client.list_pohs(attester_hex).await {
        Ok(list) => {
            eprintln!("Attester: {}", list.attester);
            eprintln!("已签发 PoH: {} 张", list.count);
            for (i, cid) in list.challenge_ids.iter().enumerate() {
                eprintln!("  [{}] {}", i, cid);
            }
        }
        Err(e) => {
            eprintln!("✗ 查询失败：{e}");
            std::process::exit(1);
        }
    }
}

/// `gyid poh-show`：解析并 pretty-print PoH 证书字段。
fn cmd_poh_show(hex_cbor: &str) {
    let bytes = hex::decode(hex_cbor).unwrap_or_else(|e| {
        eprintln!("✗ hex 解码错误：{e}");
        std::process::exit(1);
    });
    let cert = PohCertificate::from_cbor(&bytes).unwrap_or_else(|e| {
        eprintln!("✗ CBOR 解析错误：{e}");
        std::process::exit(1);
    });

    println!("=== PoH 证书 ===");
    println!("identity         = {}", hex::encode(cert.identity));
    println!("issued_at        = {}", cert.issued_at);
    println!("epoch_count      = {}", cert.epoch_count);
    println!("alpha            = {:.4}", cert.alpha);
    println!("beta             = {:.4}", cert.beta);
    println!("kappa            = {:.4}", cert.kappa);
    println!("pi               = {:.4}", cert.pi);
    println!("confidence       = {:.4}", cert.criticality_confidence);
    println!("trust            = {:.2}", cert.trust);
    println!("unique_cells     = {}", cert.unique_cells);
    println!("breadcrumb_count = {}", cert.breadcrumb_count);
    println!("validity_secs    = {}", cert.validity_secs);
    println!("nonce            = {}", hex::encode(cert.nonce));
    println!("chain_head       = {}", hex::encode(cert.chain_head));
    println!(
        "verifier_sig     = {}",
        hex::encode(cert.verifier_signature)
    );

    let policy_ok = cert.meets_policy(0.1, 20.0);
    println!(
        "\nmeets_policy(0.1, 20.0) = {}",
        if policy_ok { "PASS" } else { "FAIL" }
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 工具函数
// ═══════════════════════════════════════════════════════════════════════

/// 尝试获取 GPS 位置。无 GPS 返回 None。
///
/// 桌面 Linux 通常无 GPS 硬件；移动设备/嵌入式平台可在此接入
/// geoclue2 D-Bus / Android LocationManager / NMEA 串口等定位源。
fn get_location() -> Option<(f64, f64)> {
    None
}

fn print_poh_info(info: &gyid_shared::PohInfo) {
    println!("\n=== PoH 校验结果 ===");
    println!("identity         = {}", hex::encode(info.identity));
    println!("issued_at        = {}", info.issued_at);
    println!("epoch_count      = {}", info.epoch_count);
    println!("alpha            = {:.4}", info.alpha);
    println!("beta             = {:.4}", info.beta);
    println!("kappa            = {:.4}", info.kappa);
    println!("pi               = {:.4}", info.pi);
    println!("confidence       = {:.4}", info.criticality_confidence);
    println!("trust            = {:.2}", info.trust);
    println!("unique_cells     = {}", info.unique_cells);
    println!("breadcrumb_count = {}", info.breadcrumb_count);
    println!("validity_secs    = {}", info.validity_secs);
    println!("nonce            = {}", hex::encode(info.nonce));
    println!("chain_head       = {}", hex::encode(info.chain_head));
    println!();
    println!("fresh        = {}", info.fresh);
    println!("policy_pass  = {}", info.policy_pass);
    println!("is_trusted   = {}", info.is_trusted());
    if info.is_trusted() {
        eprintln!("\n✓ 验证通过 (FRESH + PASS)");
    } else {
        eprintln!("\n✗ 未通过");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// did:geoyuan / TIT（GYIP-0003 §5.4）
// ═══════════════════════════════════════════════════════════════════════

/// 当前 Unix 秒。
fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `gyid did show`：从 seed 生成 DID 与 DID Document。
fn cmd_did_show(
    seed_hex: &str,
    verifier: Option<&str>,
    p2p: Option<&str>,
    handle: Option<&str>,
    anchor: Option<&str>,
) {
    let key = ProtocolKey::from_seed(&parse_seed(seed_hex));
    let pubkey = key.public_bytes();

    let anchor_ref = anchor.map(|s| {
        AnchorReference::parse(s).unwrap_or_else(|| {
            eprintln!("错误：--anchor 需形如 eip155:<chain_id>:<0x地址>，收到 {s}");
            std::process::exit(1);
        })
    });
    let base = verifier.map(|v| v.trim_end_matches('/').to_string());

    let cfg = DidDocumentConfig {
        tit_endpoint: base.as_ref().map(|b| format!("{b}/v1/tit")),
        verifier_endpoint: base,
        p2p_endpoint: p2p.map(str::to_string),
        anchor: anchor_ref,
        handle: handle.map(str::to_string),
        updated: Some(now_unix_secs()),
    };
    let doc = DidDocument::build(&pubkey, &cfg);
    let json = doc.to_json_pretty().unwrap_or_else(|e| {
        eprintln!("DID Document 序列化失败：{e}");
        std::process::exit(1);
    });

    println!("did       = {}", doc.id);
    println!("pubkey    = {}", hex::encode(pubkey));
    println!("multibase = {}", trip_core::did::multibase(&pubkey));
    println!();
    println!("{json}");
}

/// `gyid did resolve`：DID → 公钥。
fn cmd_did_resolve(did: &str) {
    match trip_core::did::decode(did) {
        Ok(pubkey) => {
            println!("did       = {}", trip_core::did::encode(&pubkey));
            println!("pubkey    = {}", hex::encode(pubkey));
            println!("multibase = {}", trip_core::did::multibase(&pubkey));
        }
        Err(e) => {
            eprintln!("✗ 解析失败：{e}");
            std::process::exit(1);
        }
    }
}

/// `gyid tit issue`：签发 TIT（身份自签或 Verifier 背书）。
#[allow(clippy::too_many_arguments)] // CLI 参数直透协议字段
fn cmd_tit_issue(
    seed_hex: &str,
    identity_hex: Option<&str>,
    issuer: &str,
    epochs: u64,
    breadcrumbs: u64,
    unique_cells: u64,
    trust: f64,
    validity: u64,
    issued_at: Option<u64>,
) {
    let key = ProtocolKey::from_seed(&parse_seed(seed_hex));
    let issued = issued_at.unwrap_or_else(now_unix_secs);

    let issuer_kind = match issuer {
        "identity" => TitIssuer::Identity,
        "verifier" => TitIssuer::Verifier,
        other => {
            eprintln!("错误：--issuer 只能是 identity 或 verifier（收到 {other}）");
            std::process::exit(1);
        }
    };

    let identity = match issuer_kind {
        TitIssuer::Verifier => {
            let hex = identity_hex.unwrap_or_else(|| {
                eprintln!("错误：--issuer verifier 时必须用 --identity 指定被证明方公钥");
                std::process::exit(1);
            });
            parse_32bytes(hex, "identity")
        }
        _ => key.public_bytes(),
    };

    let claims = TitClaims {
        identity,
        epochs,
        breadcrumbs,
        unique_cells,
        trust,
        issued_at: issued,
        validity_secs: validity,
    };
    let tit = match issuer_kind {
        TitIssuer::Identity => Tit::issue_identity_signed(&key, claims),
        TitIssuer::Verifier => Tit::issue_verifier_signed(&key, claims),
        TitIssuer::Unsigned => Tit::unsigned(claims),
    };
    tit.validate().unwrap_or_else(|e| {
        eprintln!("✗ TIT 字段不合法：{e}");
        std::process::exit(1);
    });

    println!("issuer      = {}", tit.issuer.as_str());
    println!("did         = {}", tit.did());
    println!("identity    = {}", hex::encode(tit.claims.identity));
    println!("epochs      = {}", tit.claims.epochs);
    println!("breadcrumbs = {}", tit.claims.breadcrumbs);
    println!("unique_cells= {}", tit.claims.unique_cells);
    println!("trust       = {}", tit.claims.trust);
    println!("issued_at   = {}", tit.claims.issued_at);
    println!("validity    = {}", tit.claims.validity_secs);
    println!("handle_ok   = {}", tit.meets_handle_threshold());
    println!("cbor_hex    = {}", hex::encode(tit.to_cbor()));
    println!("base64url   = {}", tit.to_base64url());
}

/// `gyid tit verify`：验签 + 新鲜性（Base64url 优先，回退 hex CBOR）。
fn cmd_tit_verify(token: &str, verifier_pubkey: Option<&str>, now: Option<u64>) {
    let tit = Tit::from_base64url(token)
        .or_else(|_| {
            hex::decode(token.trim())
                .map_err(|e| trip_core::TripError::InvalidTit(format!("hex decode: {e}")))
                .and_then(|b| Tit::from_cbor(&b))
        })
        .unwrap_or_else(|e| {
            eprintln!("✗ 无法解析 TIT：{e}");
            std::process::exit(1);
        });

    let vk = verifier_pubkey.map(|h| parse_32bytes(h, "verifier_pubkey"));
    let now = now.unwrap_or_else(now_unix_secs);

    println!("issuer      = {}", tit.issuer.as_str());
    println!("did         = {}", tit.did());
    println!("epochs      = {}", tit.claims.epochs);
    println!("breadcrumbs = {}", tit.claims.breadcrumbs);
    println!("unique_cells= {}", tit.claims.unique_cells);
    println!("trust       = {}", tit.claims.trust);
    println!("issued_at   = {}", tit.claims.issued_at);
    println!("validity    = {}", tit.claims.validity_secs);
    println!("handle_ok   = {}", tit.meets_handle_threshold());

    match tit.verify(vk.as_ref(), now) {
        Ok(()) => eprintln!("\n✓ TIT 验签通过且未过期"),
        Err(e) => {
            eprintln!("\n✗ 校验失败：{e}");
            std::process::exit(1);
        }
    }
}

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

// ═══════════════════════════════════════════════════════════════════════
// 高级命令实现（保留原有逻辑）
// ═══════════════════════════════════════════════════════════════════════

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
