//! 截断 Levy 飞行模型（draft-04 §8.1 Truncated Levy Flights 与
//! §7.3 Levy-PSD Bridge）。
//!
//! 位移分布 `P(Δr) ∝ Δr^(-β) · exp(-Δr/κ)`，β 幂律指数（人类典型
//! 1.50–1.90），κ 指数截断距离（km），下界 r_min（空间量化地板）。
//!
//! ## 数值约定（跨实现确定性）
//!
//! 归一化常数与 CDF 依赖上不完全幂指数积分
//! `K(u; β) = ∫_u^∞ t^(-β) e^(-t) dt`（u > 0，对任意 β 收敛）。实现采用
//! 代换 t = u·e^s：
//!
//! ```text
//! K(u; β) = u^(1-β) · ∫_0^{L} e^{((1-β)·s - u·e^s)} ds,
//! L = ln((u + TAIL_SPAN) / u),  TAIL_SPAN = 40
//! ```
//!
//! 积分用固定 512 节点复化 Simpson；尾衰减由 `e^(-u·e^s)` 保证
//! （s = L 处 e^(-(u+40)) ≈ 4e-18）。同一算法在任何语言中可逐位复现，
//! 偏差应 < 1e-9。
//!
//! MLE 为确定性网格搜索 + 两轮局部细化（不依赖外部优化器）。

use rand::Rng;

use crate::error::{Result, TripError};

/// 积分尾跨度（无量纲，作用于 t = r/κ 空间）。
const TAIL_SPAN: f64 = 40.0;
/// Simpson 积分节点数（偶数）。
const INTEG_NODES: usize = 512;
/// β 网格：[1.0, 3.0]，步长 0.02。
const BETA_GRID_STEP: f64 = 0.02;
/// κ 网格：对数空间 [0.05, 500] km，40 点。
const KAPPA_GRID_POINTS: usize = 40;
const KAPPA_MIN: f64 = 0.05;
const KAPPA_MAX: f64 = 500.0;
/// 桥校验的 g 值范围（草案 §7.3.3 步骤 4）。
pub const BRIDGE_G_MIN: f64 = 0.3;
/// 见 [`BRIDGE_G_MIN`]。
pub const BRIDGE_G_MAX: f64 = 0.7;

/// 截断 Levy 分布参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevyParams {
    /// 幂律指数 β（人类典型 1.50–1.90）。
    pub beta: f64,
    /// 指数截断距离 κ（km）。
    pub kappa: f64,
    /// 分布下界 r_min（km；空间量化地板）。
    pub r_min: f64,
}

impl LevyParams {
    /// 构造并校验参数（β > 1，κ > 0，r_min > 0）。
    pub fn new(beta: f64, kappa: f64, r_min: f64) -> Result<Self> {
        if !(beta > 1.0 && beta < 3.0) {
            return Err(TripError::LevyFit(format!(
                "beta must be in (1, 3), got {beta}"
            )));
        }
        if !(kappa > 0.0 && r_min > 0.0) {
            return Err(TripError::LevyFit(format!(
                "kappa and r_min must be positive, got kappa={kappa}, r_min={r_min}"
            )));
        }
        Ok(Self { beta, kappa, r_min })
    }

    /// 归一化常数 Z = ∫_{r_min}^∞ r^(-β) e^(-r/κ) dr。
    pub fn normalization(&self) -> f64 {
        (self.kappa).powf(1.0 - self.beta) * k_upper(self.r_min / self.kappa, self.beta)
    }

    /// 概率密度（未截断头部；用于报告与阈值展示）。
    pub fn pdf(&self, r: f64) -> f64 {
        if r < self.r_min {
            return 0.0;
        }
        let z = self.normalization();
        r.powf(-self.beta) * (-r / self.kappa).exp() / z
    }

    /// CDF：F(r) = (K(r_min/κ) − K(r/κ)) / K(r_min/κ)。
    pub fn cdf(&self, r: f64) -> f64 {
        if r <= self.r_min {
            return 0.0;
        }
        let k0 = k_upper(self.r_min / self.kappa, self.beta);
        let kr = k_upper(r / self.kappa, self.beta);
        ((k0 - kr) / k0).clamp(0.0, 1.0)
    }

    /// 分位数（二分法，120 轮，确定性）。
    pub fn quantile(&self, p: f64) -> f64 {
        let mut lo = self.r_min;
        let mut hi = self.r_min + TAIL_SPAN * self.kappa;
        for _ in 0..120 {
            let mid = 0.5 * (lo + hi);
            if self.cdf(mid) < p {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    }

    /// 99.9% 异常阈值（§8.1：超出即递增空间异常计数器）。
    pub fn anomaly_threshold(&self) -> f64 {
        self.quantile(0.999)
    }

    /// 由两个均匀随机数驱动的一步采样（拒绝采样）。
    ///
    /// 返回 `None` 表示该组随机数被拒绝，调用方应重新抽取。
    /// 暴露为纯函数以便任何语言用任意 PRNG 精确复现（黄金向量依赖）。
    pub fn sample_from_uniforms(&self, u1: f64, u2: f64) -> Option<f64> {
        if !((0.0..1.0).contains(&u1) && (0.0..1.0).contains(&u2)) {
            return None;
        }
        // Pareto 逆变换提出：r = r_min · u1^(-1/(β-1))，密度 ∝ r^(-β)。
        let r = self.r_min * u1.powf(-1.0 / (self.beta - 1.0));
        // 以 e^(-r/κ) 概率接受 → 目标密度 ∝ r^(-β)·e^(-r/κ)。
        if u2 < (-r / self.kappa).exp() {
            Some(r)
        } else {
            None
        }
    }

    /// 使用 `rand` Rng 的便捷采样（内部循环调用 [`Self::sample_from_uniforms`]）。
    pub fn sample<R: Rng>(&self, rng: &mut R) -> f64 {
        loop {
            let u1: f64 = rng.gen();
            let u2: f64 = rng.gen();
            if let Some(r) = self.sample_from_uniforms(u1, u2) {
                return r;
            }
        }
    }
}

/// `K(u; β) = ∫_u^∞ t^(-β) e^(-t) dt`（u > 0）。
fn k_upper(u: f64, beta: f64) -> f64 {
    debug_assert!(u > 0.0, "k_upper requires u > 0");
    let l_max = ((u + TAIL_SPAN) / u).ln();
    let h = l_max / INTEG_NODES as f64;
    let f = |s: f64| ((1.0 - beta) * s - u * s.exp()).exp();
    let mut sum = f(0.0) + f(l_max);
    for i in 1..INTEG_NODES {
        let v = f(h * i as f64);
        sum += if (i & 1) == 1 { 4.0 * v } else { 2.0 * v };
    }
    u.powf(1.0 - beta) * sum * h / 3.0
}

/// 一次 MLE 拟合的结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevyFit {
    /// 拟合的 β。
    pub beta: f64,
    /// 拟合的 κ（km）。
    pub kappa: f64,
    /// 拟合使用的下界 r_min（km）。
    pub r_min: f64,
    /// 最优对数似然。
    pub log_likelihood: f64,
    /// 参与拟合的样本数。
    pub sample_count: usize,
    /// 99.9% 异常阈值（km）。
    pub threshold_999: f64,
}

/// 对位移样本做截断 Levy MLE（草案 §8.1：逐 epoch 拟合）。
///
/// `r_min` 默认取样本最小值（数据驱动的量化地板），也可显式指定。
/// 网格：β ∈ [1.0, 3.0] 步长 0.02；κ 对数网格 40 点后两轮局部细化。
pub fn fit(displacements: &[f64], r_min_override: Option<f64>) -> Result<LevyFit> {
    if displacements.len() < 8 {
        return Err(TripError::LevyFit(format!(
            "need at least 8 displacements to fit, got {}",
            displacements.len()
        )));
    }
    if displacements.iter().any(|d| !d.is_finite() || *d <= 0.0) {
        return Err(TripError::DegenerateSignal);
    }
    let r_min = r_min_override
        .unwrap_or_else(|| displacements.iter().copied().fold(f64::INFINITY, f64::min));
    if displacements.iter().all(|d| *d <= r_min) {
        return Err(TripError::LevyFit(
            "all displacements at or below r_min; cannot fit".into(),
        ));
    }

    let sum_ln: f64 = displacements.iter().map(|r| r.ln()).sum();
    let sum_r: f64 = displacements.iter().sum();
    let n = displacements.len() as f64;

    // 目标：ℓ(β,κ) = -β·Σln r - Σr/κ - n·ln Z(β,κ)
    let loglik = |beta: f64, kappa: f64| -> f64 {
        let z = kappa.powf(1.0 - beta) * k_upper(r_min / kappa, beta);
        -beta * sum_ln - sum_r / kappa - n * z.ln()
    };

    // 1) 粗网格
    let mut best = (1.5f64, KAPPA_MIN, f64::NEG_INFINITY);
    let n_kappa = KAPPA_GRID_POINTS as i32;
    for bi in 0..=100 {
        let beta = 1.0 + BETA_GRID_STEP * bi as f64;
        // LevyParams 要求 β ∈ (1,3) 开区间；网格端点恰好落在边界上
        //（0.02·100 = 2.0 精确），跳过端点防止最终 new() 拒绝。
        if !(1.0 < beta && beta < 3.0) {
            continue;
        }
        for ki in 0..n_kappa {
            let t = ki as f64 / (n_kappa - 1) as f64;
            let kappa = KAPPA_MIN * (KAPPA_MAX / KAPPA_MIN).powf(t);
            let l = loglik(beta, kappa);
            if l > best.2 {
                best = (beta, kappa, l);
            }
        }
    }

    // 2) κ 两轮对数空间细化（β 不动：初始网格已足够细）
    let mut beta_best = best.0;
    let mut kappa_best = best.1;
    let mut l_best = best.2;
    let mut decades = 1.0;
    for _ in 0..2 {
        let mut cand = (beta_best, kappa_best, l_best);
        for ki in -5..=5_i32 {
            let kappa = kappa_best * 10f64.powf(decades * ki as f64 / 5.0);
            if !(KAPPA_MIN..=KAPPA_MAX * 10.0).contains(&kappa) {
                continue;
            }
            let l = loglik(beta_best, kappa);
            if l > cand.2 {
                cand = (beta_best, kappa, l);
            }
        }
        beta_best = cand.0;
        kappa_best = cand.1;
        l_best = cand.2;
        decades /= 5.0;
    }

    // 3) β 局部细化 ±0.02，21 点
    for bi in -10..=10_i32 {
        let beta = beta_best + 0.002 * bi as f64;
        // 严格开区间（Range::contains 对下端点是包含的，会放过 1.0）。
        if !(1.0 < beta && beta < 3.0) {
            continue;
        }
        let l = loglik(beta, kappa_best);
        if l > l_best {
            l_best = l;
            beta_best = beta;
        }
    }

    let params = LevyParams::new(beta_best, kappa_best, r_min)?;
    Ok(LevyFit {
        beta: params.beta,
        kappa: params.kappa,
        r_min,
        log_likelihood: l_best,
        sample_count: displacements.len(),
        threshold_999: params.anomaly_threshold(),
    })
}

/// Levy-PSD 桥一致性检查结果（§7.3）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BridgeCheck {
    /// 修正因子 g = α / (3 − β)。
    pub g: f64,
    /// g 是否落在 [0.3, 0.7]。
    pub consistent: bool,
}

/// 校验拟合 β 与观测 α 的桥一致性（§7.3.1：α_eff ≈ (3−β)·g，g ∈ [0.3, 0.7]）。
///
/// 不一致 MAY 表示数据质量问题或对单一指标的对抗操纵（§8.1）。
pub fn bridge_check(alpha: f64, beta: f64) -> Result<BridgeCheck> {
    if !(beta > 1.0 && beta < 3.0) {
        return Err(TripError::LevyFit(format!(
            "beta must be in (1, 3), got {beta}"
        )));
    }
    if !(alpha.is_finite()) {
        return Err(TripError::DegenerateSignal);
    }
    let g = alpha / (3.0 - beta);
    Ok(BridgeCheck {
        g,
        consistent: (BRIDGE_G_MIN..=BRIDGE_G_MAX).contains(&g),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn normalization_matches_pareto_limit() {
        // κ → ∞：Z = r_min^(1-β) / (β-1)（闭式）。
        // 512 节点 Simpson 的相对误差约 1e-7 量级，容差取 5e-7。
        let p = LevyParams::new(2.0, 1e9, 0.1).unwrap();
        let expected = 0.1f64.powf(-1.0) / 1.0; // = 10
        let z = p.normalization();
        assert!(
            (z - expected).abs() / expected < 5e-7,
            "Z = {z}, expected {expected}"
        );
    }

    #[test]
    fn normalization_monotone_in_parameters() {
        let base = LevyParams::new(1.7, 5.0, 0.1).unwrap().normalization();
        // β 增大 → 重尾变轻 → Z 增大？r^(-β) 随 β 增大在 r>1 处变小、r<1 处变大。
        // r_min = 0.1 < 1：主导贡献在 r_min 附近，r^(-β) 随 β 增大而增大 → Z 增大。
        let more_beta = LevyParams::new(1.9, 5.0, 0.1).unwrap().normalization();
        assert!(more_beta > base);
        // κ 增大 → 截断变松 → Z 增大。
        let more_kappa = LevyParams::new(1.7, 50.0, 0.1).unwrap().normalization();
        assert!(more_kappa > base);
    }

    #[test]
    fn cdf_quantile_roundtrip() {
        let p = LevyParams::new(1.7, 5.0, 0.1).unwrap();
        for &q in &[0.25, 0.5, 0.9, 0.999] {
            let r = p.quantile(q);
            let back = p.cdf(r);
            assert!((back - q).abs() < 1e-6, "q={q} r={r} cdf={back}");
        }
        assert!(p.quantile(0.999) > p.quantile(0.5));
    }

    #[test]
    fn sampler_recovers_distribution_shape() {
        let params = LevyParams::new(1.7, 4.0, 0.1).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let n = 20_000;
        let mut sum = 0.0;
        let mut over_999 = 0u32;
        let thr = params.anomaly_threshold();
        let mut values = Vec::with_capacity(n);
        for _ in 0..n {
            let r = params.sample(&mut rng);
            assert!(r >= params.r_min);
            if r > thr {
                over_999 += 1;
            }
            sum += r;
            values.push(r);
        }
        // 经验均值 vs 理论均值（数值积分），5% 容差。
        let mean_theory: f64 = {
            // E[r] = ∫ r·pdf dr；用 512 点 Simpson 在 [r_min, r_min+40κ] 上算。
            let a = params.r_min;
            let b = a + TAIL_SPAN * params.kappa;
            let m = 512;
            let h = (b - a) / m as f64;
            let f = |r: f64| r * params.pdf(r);
            let mut s = f(a) + f(b);
            for i in 1..m {
                let v = f(a + h * i as f64);
                s += if (i & 1) == 1 { 4.0 * v } else { 2.0 * v };
            }
            s * h / 3.0
        };
        let mean_emp = sum / n as f64;
        assert!(
            (mean_emp - mean_theory).abs() / mean_theory < 0.05,
            "empirical mean {mean_emp} vs theory {mean_theory}"
        );
        // 超过 99.9 分位的比例应接近 0.1%（±0.05%，采样噪声内）。
        let frac = over_999 as f64 / n as f64;
        assert!((0.0005..0.0015).contains(&frac), "tail fraction {frac}");
        // 分位数单调性
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95 = values[(0.95 * n as f64) as usize];
        assert!((p95 - params.quantile(0.95)).abs() / params.quantile(0.95) < 0.05);
    }

    #[test]
    fn fit_recovers_true_parameters() {
        let truth = LevyParams::new(1.7, 4.0, 0.1).unwrap();
        let mut rng = StdRng::seed_from_u64(7);
        let samples: Vec<f64> = (0..400).map(|_| truth.sample(&mut rng)).collect();
        let fitted = fit(&samples, None).unwrap();
        assert!(
            (fitted.beta - 1.7).abs() < 0.15,
            "fitted beta {} vs 1.7",
            fitted.beta
        );
        assert!(
            (fitted.kappa / 4.0).ln().abs() < 0.5,
            "fitted kappa {} vs 4.0",
            fitted.kappa
        );
        assert!(fitted.threshold_999 > fitted.kappa);
    }

    #[test]
    fn bridge_check_matches_spec() {
        // β=1.75（典型）→ 3-β=1.25；α=0.55 → g=0.44 ∈ [0.3,0.7] ✓
        assert!(bridge_check(0.55, 1.75).unwrap().consistent);
        // β=1.5, α=0.9 → g=0.6 ✓
        assert!(bridge_check(0.9, 1.5).unwrap().consistent);
        // β=1.9, α=0.95 → g=0.792 ✗
        assert!(!bridge_check(0.95, 1.9).unwrap().consistent);
        assert!((bridge_check(0.55, 1.75).unwrap().g - 0.44).abs() < 1e-12);
    }

    #[test]
    fn sample_from_uniforms_rejects_large_proposals() {
        let p = LevyParams::new(1.7, 1.0, 0.1).unwrap();
        // u1 → 1⁻：提出 r 接近 r_min；接受概率 ≈ e^(-0.1) ≈ 0.905 → u2=0.5 接受。
        let small = p.sample_from_uniforms(1.0 - 1e-12, 0.5).unwrap();
        assert!((small - p.r_min).abs() < 1e-3, "r = {small}");
        // u1 = 0.5：r = 0.1·0.5^(-1/0.7) ≈ 0.269 km；e^(-r/κ) ≈ 0.764 → 接受。
        let mid = p.sample_from_uniforms(0.5, 0.5).unwrap();
        assert!((mid - 0.1 * 0.5f64.powf(-1.0 / 0.7)).abs() < 1e-12);
        // u1 → 0：提出 r 巨大（重尾）；e^(-r/κ) → 0 → 拒绝。
        assert_eq!(p.sample_from_uniforms(1e-12, 0.99), None);
        assert_eq!(p.sample_from_uniforms(0.999999999, 0.99), None);
        // 边界非法。
        assert_eq!(p.sample_from_uniforms(0.0, 0.5), None);
        assert_eq!(p.sample_from_uniforms(0.5, 1.0), None);
    }

    #[test]
    fn fit_requires_enough_data() {
        let e = fit(&[0.5; 7], None).unwrap_err();
        assert!(matches!(e, TripError::LevyFit(_)));
    }
}
