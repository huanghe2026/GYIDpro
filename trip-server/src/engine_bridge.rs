//! trip-core 经典引擎的服务端桥接（阻塞纯计算）。
//!
//! 调用方（WebSocket handler）必须把本模块函数放进
//! `tokio::task::spawn_blocking`：朴素 O(N²) DFT 与 Levy 网格 MLE 是
//! CPU 密集型，不得阻塞异步 runtime（见 W5 计划容量章节）。
//!
//! 评估顺序与 `trip-core/examples/poh_pipeline.rs` 一致：
//! 行为画像 → PSD α → Levy MLE → Hamiltonian（记录）→ 信任分。
//! PSD/Levy 只取最近 [`PSD_WINDOW`] 条面包屑（对齐草案 256 高风险窗口），
//! 画像 / unique cell / 信任分仍基于全链。

use trip_core::engine::behavior::{BehavioralProfile, BreadcrumbView};
use trip_core::engine::hamiltonian::evaluate as hamiltonian_evaluate;
use trip_core::engine::levy::fit as levy_fit;
use trip_core::engine::psd::{displacements_from_cells, psd_alpha};
use trip_core::engine::trust::{alpha_in_bio_range, trust_score, TrustInput};
use trip_core::tit::{Tit, TitClaims};
use trip_core::{Breadcrumb, PohCertificate, ProtocolKey, TripError};

/// PSD/Levy 评估窗口（面包屑条数；草案高风险决策上限 256）。
pub const PSD_WINDOW: usize = 256;
/// 草案默认每条 epoch 的面包屑数（MVP 未独立接收 epoch，按此近似计数）。
const EPOCH_SIZE: u64 = 100;

/// 一次身份评估的聚合结果（PoH 证书的统计字段来源）。
#[derive(Debug, Clone, Copy)]
pub struct EvalResult {
    pub alpha: f64,
    pub beta: f64,
    pub kappa: f64,
    pub pi: f64,
    pub confidence: f64,
    pub trust: f64,
    pub unique_cells: u64,
    pub breadcrumb_count: u64,
    pub epoch_count: u64,
    pub chain_head: [u8; 32],
}

/// 对一条已验证完整面包屑链跑全部经典引擎，产出评估结果。
///
/// `crumbs` 必须非空、且此前已通过
/// [`trip_core::ChainRules::verify`](trip_core::ChainRules)（签名/链完整可信）。
pub fn evaluate_identity(crumbs: &[Breadcrumb]) -> Result<EvalResult, TripError> {
    if crumbs.is_empty() {
        return Err(TripError::InvalidChain("empty breadcrumb chain".into()));
    }

    // ---- 行为画像（全链）----
    let views: Vec<BreadcrumbView> = crumbs
        .iter()
        .map(|bc| BreadcrumbView {
            ts: bc.timestamp as i64,
            cell: bc.h3_cell,
            prev_hash: bc.prev_hash,
            block_hash: bc.block_hash(),
            imu_present: bc.meta.imu_present,
        })
        .collect();
    let profile = BehavioralProfile::from_breadcrumbs(&views)?;

    // ---- PSD/Levy（最近窗口）----
    let window_start = crumbs.len().saturating_sub(PSD_WINDOW);
    let window_cells: Vec<u64> = crumbs[window_start..].iter().map(|bc| bc.h3_cell).collect();
    let displacements = displacements_from_cells(&window_cells)?;
    let psd = psd_alpha(&displacements)?;
    let levy = levy_fit(&displacements, None)?;

    // ---- Hamiltonian（评估最后一条；结果仅记录，不进 PoH 15 字段）----
    if views.len() >= 2 {
        let last = views.last().expect("len >= 2");
        let prev = &views[views.len() - 2];
        let last_disp = *displacements.last().unwrap_or(&0.0);
        let report = hamiltonian_evaluate(
            &profile,
            last,
            last_disp,
            &[],
            Some((0.1, 0.05)),
            prev.ts,
            prev.cell,
            true,
        );
        tracing::debug!(
            alpha = psd.alpha,
            h_total = report.total,
            alert = report.alert.as_str(),
            "hamiltonian evaluated"
        );
    }

    // ---- 可预测性 Π ----
    let pi = predictability(&profile);

    // ---- 信任分（全链计数）----
    let last_ts = crumbs.last().expect("non-empty").timestamp as i64;
    let days_since_first = ((last_ts - profile.first_ts).max(0) as f64) / 86400.0;
    let alpha_ok = alpha_in_bio_range(psd.alpha);
    let trust_input = TrustInput {
        breadcrumb_count: crumbs.len(),
        unique_cells: profile.unique_cells,
        days_since_first,
        chain_integrity: true,
    };
    let trust = trust_score(&trust_input, alpha_ok);

    let chain_head = crumbs.last().expect("non-empty").block_hash();

    Ok(EvalResult {
        alpha: psd.alpha,
        beta: levy.beta,
        kappa: levy.kappa,
        pi,
        confidence: psd.confidence,
        trust,
        unique_cells: profile.unique_cells as u64,
        breadcrumb_count: crumbs.len() as u64,
        epoch_count: crumbs.len() as u64 / EPOCH_SIZE,
        chain_head,
    })
}

/// 基于评估结果签发绑定 RP nonce 的 PoH 证书（Verifier 签名）。
#[allow(clippy::too_many_arguments)] // 透传 PoH 协议字段
pub fn issue_poh(
    verifier: &ProtocolKey,
    attester: [u8; 32],
    eval: &EvalResult,
    rp_nonce: [u8; 16],
    issued_at: u64,
    validity_secs: u64,
) -> PohCertificate {
    PohCertificate::issue(
        verifier,
        attester,
        issued_at,
        eval.epoch_count,
        eval.alpha,
        eval.beta,
        eval.kappa,
        eval.pi,
        eval.confidence,
        eval.trust,
        eval.unique_cells,
        eval.breadcrumb_count,
        validity_secs,
        rp_nonce,
        eval.chain_head,
    )
}

/// 基于评估结果签发 **Verifier 背书**的 TIT（GYIP-0003 §5.4）。
///
/// 与 PoH 的分工：PoH 绑定一次性 RP nonce 且携带完整统计指数；TIT 长期
/// 有效、可放进二维码/DID Document，用于展示与发现。两者都由 Verifier
/// 长密钥签名，RP 用同一份 `/.well-known/verifier.json` 公钥验签。
pub fn issue_tit(
    verifier: &ProtocolKey,
    attester: [u8; 32],
    eval: &EvalResult,
    issued_at: u64,
    validity_secs: u64,
) -> Tit {
    Tit::issue_verifier_signed(
        verifier,
        TitClaims {
            identity: attester,
            epochs: eval.epoch_count,
            breadcrumbs: eval.breadcrumb_count,
            unique_cells: eval.unique_cells,
            trust: eval.trust,
            issued_at,
            validity_secs,
        },
    )
}

/// 由 Markov 转移矩阵估算可预测性 Π：按行样本量加权的"行内最大转移概率"。
///
/// 无锚点（轨迹无重复驻留，如仿真轨迹）时返回保守 0.5。该字段仅记录进
/// PoH，RP 策略门不使用它。
fn predictability(profile: &BehavioralProfile) -> f64 {
    if profile.anchors.is_empty() {
        return 0.5;
    }
    let mut total: usize = 0;
    let mut weighted = 0.0;
    for tos in profile.transitions.values() {
        let row_total: usize = tos.values().sum();
        if row_total == 0 {
            continue;
        }
        let row_max = *tos.values().max().expect("non-empty row") as f64;
        weighted += (row_max / row_total as f64) * row_total as f64;
        total += row_total;
    }
    if total == 0 {
        0.5
    } else {
        weighted / total as f64
    }
}
