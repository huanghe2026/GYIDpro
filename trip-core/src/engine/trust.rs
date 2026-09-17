//! 信任分（draft-04 §10 Trust Scoring）。
//!
//! `T = 0.40·min(n/200,1) + 0.30·min(unique/50,1) + 0.20·min(days/365,1)
//! + 0.10·chain_integrity`，T ∈ [0, 100]。
//!
//! - handle 声明需 n ≥ 100 且 T ≥ 20；
//! - 临界测试失败（α ∉ [0.30, 0.80]）时 T 封顶 50。

use crate::engine::psd::{BIO_ALPHA_MAX, BIO_ALPHA_MIN};

/// 信任分权重（草案 §10）。
const W_COUNT: f64 = 0.40;
const W_UNIQUE: f64 = 0.30;
const W_DAYS: f64 = 0.20;
const W_INTEGRITY: f64 = 0.10;

/// 面包屑数成熟阈值。
const COUNT_THRESHOLD: usize = 200;
/// unique cell 成熟阈值。
const UNIQUE_THRESHOLD: usize = 50;
/// 天数成熟阈值。
const DAYS_THRESHOLD: f64 = 365.0;

/// handle 声明的最小面包屑数。
pub const HANDLE_MIN_BREADCRUMBS: usize = 100;
/// handle 声明的最小信任分。
pub const HANDLE_MIN_TRUST: f64 = 20.0;
/// 临界测试失败时的信任分封顶。
pub const TRUST_CAP_ON_CRIT_FAIL: f64 = 50.0;

/// 信任分输入。
#[derive(Debug, Clone, Copy)]
pub struct TrustInput {
    /// 面包屑总数。
    pub breadcrumb_count: usize,
    /// unique cell 数。
    pub unique_cells: usize,
    /// 首条面包屑到现在的天数。
    pub days_since_first: f64,
    /// 链完整性通过 = 1.0，否则 0.0。
    pub chain_integrity: bool,
}

/// 计算信任分 T（百分比 [0, 100]）。
///
/// - `alpha_criticality_ok`：PSD α 是否落入 [0.30, 0.80]（生物区间）。
///   false 时 T 封顶 50（§10）。
pub fn trust_score(input: &TrustInput, alpha_criticality_ok: bool) -> f64 {
    let maturity_count = (input.breadcrumb_count as f64 / COUNT_THRESHOLD as f64).min(1.0);
    let maturity_unique = (input.unique_cells as f64 / UNIQUE_THRESHOLD as f64).min(1.0);
    let maturity_days = (input.days_since_first / DAYS_THRESHOLD).min(1.0);
    let integrity = if input.chain_integrity { 1.0 } else { 0.0 };

    let mut t = W_COUNT * maturity_count
        + W_UNIQUE * maturity_unique
        + W_DAYS * maturity_days
        + W_INTEGRITY * integrity;
    t *= 100.0;

    if !alpha_criticality_ok {
        t = t.min(TRUST_CAP_ON_CRIT_FAIL);
    }

    t
}

/// 判断是否满足 handle 声明门槛（n ≥ 100 且 T ≥ 20）。
pub fn can_claim_handle(input: &TrustInput, alpha_criticality_ok: bool) -> bool {
    input.breadcrumb_count >= HANDLE_MIN_BREADCRUMBS
        && trust_score(input, alpha_criticality_ok) >= HANDLE_MIN_TRUST
}

/// 判断 α 是否在生物区间（§7.1）。
pub fn alpha_in_bio_range(alpha: f64) -> bool {
    (BIO_ALPHA_MIN..=BIO_ALPHA_MAX).contains(&alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_input(n: usize, unique: usize, days: f64) -> TrustInput {
        TrustInput {
            breadcrumb_count: n,
            unique_cells: unique,
            days_since_first: days,
            chain_integrity: true,
        }
    }

    #[test]
    fn full_maturity_yields_100() {
        let t = trust_score(&full_input(500, 100, 500.0), true);
        assert!((t - 100.0).abs() < 1e-9, "t = {t}");
    }

    #[test]
    fn bootstrap_yields_low_trust() {
        let t = trust_score(&full_input(10, 5, 1.0), true);
        assert!(t < 20.0, "bootstrap trust should be < 20, got {t}");
    }

    #[test]
    fn handle_threshold_at_100_crumbs() {
        // n=100, unique=50, days=365 → T = 0.40·0.5 + 0.30·1 + 0.20·1 + 0.10·1 = 80
        let t = trust_score(&full_input(100, 50, 365.0), true);
        assert!((t - 80.0).abs() < 1e-9, "t = {t}");
        assert!(can_claim_handle(&full_input(100, 50, 365.0), true));
    }

    #[test]
    fn handle_threshold_not_met_below_100() {
        assert!(!can_claim_handle(&full_input(99, 50, 365.0), true));
    }

    #[test]
    fn criticality_fail_caps_at_50() {
        let t = trust_score(&full_input(500, 100, 500.0), false);
        assert!((t - 50.0).abs() < 1e-9, "capped t = {t}");
    }

    #[test]
    fn criticality_fail_does_not_reduce_below_natural() {
        // 小 profile 自然 T < 50，封顶不影响
        let t = trust_score(&full_input(10, 5, 1.0), false);
        assert!(t < 50.0);
    }

    #[test]
    fn chain_broken_reduces_trust() {
        let ok = trust_score(&full_input(500, 100, 500.0), true);
        let broken = trust_score(
            &TrustInput {
                breadcrumb_count: 500,
                unique_cells: 100,
                days_since_first: 500.0,
                chain_integrity: false,
            },
            true,
        );
        assert!(broken < ok, "broken = {broken}, ok = {ok}");
    }

    #[test]
    fn alpha_in_bio_range_works() {
        assert!(alpha_in_bio_range(0.30));
        assert!(alpha_in_bio_range(0.55));
        assert!(alpha_in_bio_range(0.80));
        assert!(!alpha_in_bio_range(0.29));
        assert!(!alpha_in_bio_range(0.81));
        assert!(!alpha_in_bio_range(0.0));
    }
}
