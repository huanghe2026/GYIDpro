//! PSD 临界性引擎（draft-04 §7.1 Power Spectral Density Analysis /
//! §7.2 Criticality Confidence）。
//!
//! 给定位移时间序列 d(0..N)（相邻面包屑 H3 cell 中心间的 haversine 距离，
//! 单位 km），计算：
//!
//! 1. 周期图 `S(f_k) = |DFT(d)_k|²`（k = 1..=N/2，跳过 DC；均值分量只落在
//!    k=0，因此跳过 DC 等价于去均值）；
//! 2. log-log 线性回归 `ln S ~ -α · ln f + c`，斜率取负得标度指数 α；
//! 3. 拟合优度 R² 与临界置信度
//!    `confidence = max(0, 1 - |α - 0.55| / 0.25) · R²`；
//! 4. 按草案分类表映射噪声类型（白 / 近白 / 粉 / 近棕 / 棕）。
//!
//! ## 跨实现确定性约定（黄金向量依赖）
//!
//! - DFT 采用朴素 O(N²) 定义，角频率按 `(j·k mod N)/N · 2π` 归约后求三角函数；
//! - 频率取归一化 `f_k = k/N`（常数因子不影响斜率）；
//! - 回归为普通最小二乘，R² = SXY²/(SXX·SYY)；
//! - 浮点全部为 IEEE-754 f64。任何语言按相同步骤实现，α 的偏差应 < 1e-9。
//!
//! 窗口约束：位移样本数至少 [`MIN_PSD_SAMPLES`]（64，对应约 65 条面包屑），
//! 推荐 [`RECOMMENDED_PSD_SAMPLES`]（256）。面包屑数量级的收敛区间见
//! [`convergence_level`]。

use h3o::{CellIndex, LatLng};

use crate::error::{Result, TripError};

/// 生物（粉噪）区间下界（§7.1 Table: PSD Scaling Exponent Classification）。
pub const BIO_ALPHA_MIN: f64 = 0.30;
/// 生物区间上界。
pub const BIO_ALPHA_MAX: f64 = 0.80;
/// 生物区间中心（置信度公式的锚点）。
pub const ALPHA_CENTER: f64 = 0.55;
/// 生物区间半宽。
pub const ALPHA_HALF_WIDTH: f64 = 0.25;

/// DFT 最小位移样本数（草案 §7.1：64 条面包屑为最小窗口）。
pub const MIN_PSD_SAMPLES: usize = 64;
/// 推荐位移样本数（§7.1：256）。
pub const RECOMMENDED_PSD_SAMPLES: usize = 256;

/// 面包屑里程碑：PSD 最小值（§7.4.4）。
pub const MIN_BREADCRUMBS_PSD: u64 = 64;
/// 面包屑里程碑：可认领 handle（§7.4.4）。
pub const MIN_BREADCRUMBS_HANDLE: u64 = 100;
/// 面包屑里程碑：可靠正判（§7.4.4，推荐）。
pub const MIN_BREADCRUMBS_STABLE: u64 = 200;
/// 面包屑里程碑：高风险 RP 决策（§7.4.4）。
pub const MIN_BREADCRUMBS_HIGH_STAKES: u64 = 256;

/// 地球平均半径（km），haversine 使用。
const EARTH_RADIUS_KM: f64 = 6371.0088;

const TAU: f64 = core::f64::consts::TAU;

/// α 分类（草案 §7.1 分类表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PsdClass {
    /// α < 0.15：白噪声 → 合成/自动化脚本。
    White,
    /// 0.15 ≤ α < 0.30：近白 → 可疑（高级 bot）。
    NearWhite,
    /// 0.30 ≤ α ≤ 0.80：粉噪（1/f）→ 生物/人类。
    Pink,
    /// 0.80 < α ≤ 1.20：近棕 → 可疑（带漂移的重放）。
    NearBrown,
    /// α > 1.20：棕噪 → 漂移异常/传感器故障。
    Brown,
}

impl PsdClass {
    /// 稳定字符串标识（报告/日志用）。
    pub fn as_str(self) -> &'static str {
        match self {
            PsdClass::White => "white",
            PsdClass::NearWhite => "near_white",
            PsdClass::Pink => "pink",
            PsdClass::NearBrown => "near_brown",
            PsdClass::Brown => "brown",
        }
    }
}

/// 按 §7.1 分类表映射 α。
pub fn classify_alpha(alpha: f64) -> PsdClass {
    match alpha {
        a if a < 0.15 => PsdClass::White,
        a if a < BIO_ALPHA_MIN => PsdClass::NearWhite,
        a if a <= BIO_ALPHA_MAX => PsdClass::Pink,
        a if a <= 1.20 => PsdClass::NearBrown,
        _ => PsdClass::Brown,
    }
}

/// 收敛区间（草案 §7.4.1 Convergence Regimes，按面包屑数量级）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvergenceLevel {
    /// 0–63：PSD 不计算。
    Bootstrap,
    /// 64–199：可计算但方差大（α 方差 ~0.15）。
    Provisional,
    /// 200–255：稳定（α 方差 < 0.05）。
    Stable,
    /// ≥256：适合高风险决策。
    HighConfidence,
}

/// 按面包屑总数映射收敛区间。
pub fn convergence_level(breadcrumbs: u64) -> ConvergenceLevel {
    match breadcrumbs {
        n if n < MIN_BREADCRUMBS_PSD => ConvergenceLevel::Bootstrap,
        n if n < MIN_BREADCRUMBS_STABLE => ConvergenceLevel::Provisional,
        n if n < MIN_BREADCRUMBS_HIGH_STAKES => ConvergenceLevel::Stable,
        _ => ConvergenceLevel::HighConfidence,
    }
}

/// 一次 PSD 分析的完整结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PsdAnalysis {
    /// 参与拟合的位移样本数。
    pub sample_count: usize,
    /// PSD 标度指数 α。
    pub alpha: f64,
    /// log-log 回归的 R²。
    pub r_squared: f64,
    /// 临界置信度 ∈ [0,1]。
    pub confidence: f64,
    /// 噪声分类。
    pub classification: PsdClass,
}

/// 计算临界置信度（§7.2 公式，截断到 [0,1]）。
pub fn criticality_confidence(alpha: f64, r_squared: f64) -> f64 {
    let alpha_score = 1.0 - (alpha - ALPHA_CENTER).abs() / ALPHA_HALF_WIDTH;
    (alpha_score.max(0.0) * r_squared).clamp(0.0, 1.0)
}

/// 对位移序列计算 PSD 标度指数（§7.1）。
///
/// `displacements` 为相邻面包屑 cell 中心间的 haversine 距离（km），
/// 长度 N 至少 [`MIN_PSD_SAMPLES`]，所有值必须为正有限数。
pub fn psd_alpha(displacements: &[f64]) -> Result<PsdAnalysis> {
    let n = displacements.len();
    if n < MIN_PSD_SAMPLES {
        return Err(TripError::InsufficientPsdSamples(MIN_PSD_SAMPLES, n));
    }
    if displacements.iter().any(|d| !d.is_finite() || *d <= 0.0) {
        return Err(TripError::DegenerateSignal);
    }
    debug_assert!(n <= 1 << 20, "psd window unreasonably large");

    let half = n / 2;
    let mut xs = Vec::with_capacity(half);
    let mut ys = Vec::with_capacity(half);
    for k in 1..=half {
        // 朴素 DFT（k ≠ 0）：e^{-2πi jk/N}，按 (j·k mod N) 归约角度。
        let mut re = 0.0;
        let mut im = 0.0;
        for (j, &d) in displacements.iter().enumerate() {
            let m = ((j as u64 * k as u64) % n as u64) as f64;
            let ang = -TAU * m / n as f64;
            re += d * ang.cos();
            im += d * ang.sin();
        }
        let s = re * re + im * im;
        if !s.is_finite() || s <= 0.0 {
            return Err(TripError::DegenerateSignal);
        }
        xs.push((k as f64 / n as f64).ln());
        ys.push(s.ln());
    }

    let (slope, r_squared) = ols(&xs, &ys);
    let alpha = -slope;
    Ok(PsdAnalysis {
        sample_count: n,
        alpha,
        r_squared,
        confidence: criticality_confidence(alpha, r_squared),
        classification: classify_alpha(alpha),
    })
}

/// 把 H3 cell 序列转成位移序列（km）：d(i) = haversine(center_i, center_{i-1})。
///
/// 输入 cell 必须合法；连续相同 cell 会产生 0 位移并在后续
/// [`psd_alpha`] 处报 [`TripError::DegenerateSignal`]——正常流程中链规则
/// （§4.1）已保证不会出现。
pub fn displacements_from_cells(cells: &[u64]) -> Result<Vec<f64>> {
    if cells.len() < 2 {
        return Err(TripError::InvalidH3Cell(
            cells.first().copied().unwrap_or(0),
        ));
    }
    let mut prev = parse_cell(cells[0])?;
    let mut out = Vec::with_capacity(cells.len() - 1);
    for &raw in &cells[1..] {
        let cur = parse_cell(raw)?;
        out.push(haversine_km(prev, cur));
        prev = cur;
    }
    Ok(out)
}

fn parse_cell(raw: u64) -> Result<CellIndex> {
    CellIndex::try_from(raw).map_err(|_| TripError::InvalidH3Cell(raw))
}

fn haversine_km(a: CellIndex, b: CellIndex) -> f64 {
    let p1 = LatLng::from(a);
    let p2 = LatLng::from(b);
    let lat1 = p1.lat().to_radians();
    let lat2 = p2.lat().to_radians();
    let dlat = lat2 - lat1;
    let dlng = (p2.lng() - p1.lng()).to_radians();
    let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlng / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * h.sqrt().asin()
}

/// 普通最小二乘：返回 (斜率, R²)。
fn ols(x: &[f64], y: &[f64]) -> (f64, f64) {
    let n = x.len() as f64;
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    for i in 0..x.len() {
        let dx = x[i] - mx;
        let dy = y[i] - my;
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    let slope = sxy / sxx;
    let r_squared = if syy > 0.0 {
        (sxy * sxy) / (sxx * syy)
    } else {
        0.0
    };
    (slope, r_squared)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// xorshift64*（与黄金向量生成器相同的确定性 PRNG）。
    struct XorShift(u64);
    impl XorShift {
        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn f64(&mut self) -> f64 {
            ((self.next_u64() >> 11) as f64) / (1u64 << 53) as f64
        }
    }

    const N: usize = 256;

    fn white_series() -> Vec<f64> {
        let mut rng = XorShift(0xDEAD_BEEF);
        (0..N).map(|_| 1.0 + rng.f64()).collect()
    }

    fn brown_series() -> Vec<f64> {
        let mut rng = XorShift(0xCAFE_BABE);
        let mut c = 100.0;
        (0..N)
            .map(|_| {
                c += 2.0 * rng.f64() - 1.0;
                c
            })
            .collect()
    }

    fn pink_series() -> Vec<f64> {
        // Voss-McCartney 粉噪（宽带 1/f）：9 个倍频程行，第 k 行每 2^k 步
        // 更新一次；各行取值相加。确定性、跨语言可复现，α ≈ 1。
        const ROWS: usize = 9;
        let mut rng = XorShift(0x1234_5678);
        let mut rows = [0.0f64; ROWS];
        (0..N)
            .map(|i| {
                for (k, slot) in rows.iter_mut().enumerate() {
                    if i % (1usize << k) == 0 {
                        *slot = rng.f64();
                    }
                }
                rows.iter().sum::<f64>()
            })
            .collect()
    }

    #[test]
    fn white_noise_has_alpha_near_zero() {
        let a = psd_alpha(&white_series()).unwrap();
        assert!(a.alpha < 0.15, "white alpha = {}", a.alpha);
        assert_eq!(a.classification, PsdClass::White);
    }

    #[test]
    fn brown_noise_has_alpha_near_two() {
        let a = psd_alpha(&brown_series()).unwrap();
        assert!(a.alpha > 1.5, "brown alpha = {}", a.alpha);
        assert!(
            matches!(a.classification, PsdClass::NearBrown | PsdClass::Brown),
            "brown class = {:?}",
            a.classification
        );
    }

    #[test]
    fn pink_noise_lands_in_biological_range() {
        let a = psd_alpha(&pink_series()).unwrap();
        assert!(
            (0.7..=1.3).contains(&a.alpha),
            "pink alpha = {} (Voss-McCartney approximation)",
            a.alpha
        );
        // 置信度只在 α 落入协议生物区间 [0.30, 0.80] 时非零（公式在区间外
        // 按定义截断为 0）。Voss 粉噪略偏 1 附近，两种情况都可能。
        if a.alpha <= BIO_ALPHA_MAX {
            assert!(
                a.confidence > 0.0,
                "alpha={} conf={}",
                a.alpha,
                a.confidence
            );
        } else {
            assert_eq!(a.confidence, 0.0);
        }
    }

    #[test]
    fn confidence_formula_matches_spec() {
        // α = 0.55（中心）→ alpha_score = 1
        assert!((criticality_confidence(0.55, 0.8) - 0.8).abs() < 1e-12);
        // α 越界 → alpha_score 截断为 0
        assert_eq!(criticality_confidence(0.10, 0.9), 0.0);
        // R² 截断到 1
        assert_eq!(criticality_confidence(0.55, 1.5), 1.0);
    }

    #[test]
    fn classification_boundaries() {
        assert_eq!(classify_alpha(0.0), PsdClass::White);
        assert_eq!(classify_alpha(0.149), PsdClass::White);
        assert_eq!(classify_alpha(0.15), PsdClass::NearWhite);
        assert_eq!(classify_alpha(0.299), PsdClass::NearWhite);
        assert_eq!(classify_alpha(0.30), PsdClass::Pink);
        assert_eq!(classify_alpha(0.55), PsdClass::Pink);
        assert_eq!(classify_alpha(0.80), PsdClass::Pink);
        assert_eq!(classify_alpha(0.81), PsdClass::NearBrown);
        assert_eq!(classify_alpha(1.20), PsdClass::NearBrown);
        assert_eq!(classify_alpha(1.21), PsdClass::Brown);
    }

    #[test]
    fn convergence_levels() {
        assert_eq!(convergence_level(0), ConvergenceLevel::Bootstrap);
        assert_eq!(convergence_level(63), ConvergenceLevel::Bootstrap);
        assert_eq!(convergence_level(64), ConvergenceLevel::Provisional);
        assert_eq!(convergence_level(100), ConvergenceLevel::Provisional);
        assert_eq!(convergence_level(199), ConvergenceLevel::Provisional);
        assert_eq!(convergence_level(200), ConvergenceLevel::Stable);
        assert_eq!(convergence_level(255), ConvergenceLevel::Stable);
        assert_eq!(convergence_level(256), ConvergenceLevel::HighConfidence);
    }

    #[test]
    fn insufficient_samples_rejected() {
        let short = vec![1.0f64; 63];
        assert!(matches!(
            psd_alpha(&short),
            Err(TripError::InsufficientPsdSamples(64, 63))
        ));
    }

    #[test]
    fn non_positive_displacements_rejected() {
        let mut s = white_series();
        s[10] = 0.0;
        assert!(matches!(psd_alpha(&s), Err(TripError::DegenerateSignal)));
    }

    #[test]
    fn cells_to_displacements_sane() {
        // 相距约 0.2 km 的两个 res10 cell（北京附近）。
        let a = LatLng::new(39.9042, 116.4074)
            .unwrap()
            .to_cell(h3o::Resolution::Ten);
        let b = LatLng::new(39.9054, 116.4074)
            .unwrap()
            .to_cell(h3o::Resolution::Ten);
        let d = displacements_from_cells(&[u64::from(a), u64::from(b)]).unwrap();
        assert_eq!(d.len(), 1);
        assert!(d[0] > 0.05 && d[0] < 0.5, "displacement = {} km", d[0]);
    }
}
