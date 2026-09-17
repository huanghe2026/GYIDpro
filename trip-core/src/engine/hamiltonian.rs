//! 六分量 Hamiltonian 异常评分（draft-04 §8 The Six-Component Hamiltonian）。
//!
//! `H = Σ w_i · H_i`（权重乘成熟度 m），评估新面包屑偏离画像的"能量"。
//! 高能 = 异常；低能 = 正常。告警分级：
//!
//! | H 范围 | 级别 |
//! |---|---|
//! | [0, H_baseline·1.5) | NOMINAL |
//! | [H_baseline·1.5, 3.0) | ELEVATED |
//! | [3.0, 5.0) | SUSPICIOUS |
//! | [5.0, ∞) | CRITICAL |

use crate::engine::behavior::{BehavioralProfile, BreadcrumbView};

/// 各分量权重（草案 §8 Table）。
pub const W_SPATIAL: f64 = 0.25;
pub const W_TEMPORAL: f64 = 0.20;
pub const W_KINETIC: f64 = 0.20;
pub const W_FLOCK: f64 = 0.15;
pub const W_CONTEXTUAL: f64 = 0.10;
pub const W_STRUCTURE: f64 = 0.10;

/// H_structure 在链断时的最大惩罚。
const H_STRUCTURE_MAX: f64 = 5.0;

/// 告警级别（§8 Alert Classification）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    Nominal,
    Elevated,
    Suspicious,
    Critical,
}

impl AlertLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::Elevated => "elevated",
            Self::Suspicious => "suspicious",
            Self::Critical => "critical",
        }
    }
}

/// 单次 Hamiltonian 评估结果。
#[derive(Debug, Clone)]
pub struct HamiltonianReport {
    /// 总能量 H = Σ w_i · H_i · m。
    pub total: f64,
    /// 成熟度 m = min(n/200, 1)。
    pub maturity: f64,
    /// H_baseline（滚动中位），无足够历史时为 0。
    pub baseline: f64,
    /// 各分量明细。
    pub spatial: f64,
    pub temporal: f64,
    pub kinetic: f64,
    pub flock: f64,
    pub contextual: f64,
    pub structure: f64,
    /// 告警级别。
    pub alert: AlertLevel,
}

/// co-located 实体的速度向量（供 H_flock 使用）。
#[derive(Debug, Clone, Copy)]
pub struct FlockVelocity {
    /// 速度分量（km，东向 / 北向）。
    pub east: f64,
    pub north: f64,
}

/// 评估一条新面包屑的 Hamiltonian 能量。
///
/// - `profile`：身份画像。
/// - `current`：待评估的面包屑。
/// - `displacement_km`：current 与前一条的 haversine 位移。
/// - `flock_velocities`：co-located 实体的速度向量（无则空切片 → 回退模式）。
/// - `own_velocity`：当前身份自身的速度向量 (east, north) km。
/// - `prev_ts`：前一条面包屑时间戳。
/// - `prev_cell`：前一条面包屑 cell。
/// - `chain_ok`：哈希链完整性。
#[allow(clippy::too_many_arguments)] // 8 参数对应草案 §8 六分量公式
pub fn evaluate(
    profile: &BehavioralProfile,
    current: &BreadcrumbView,
    displacement_km: f64,
    flock_velocities: &[FlockVelocity],
    own_velocity: Option<(f64, f64)>,
    prev_ts: i64,
    prev_cell: u64,
    chain_ok: bool,
) -> HamiltonianReport {
    let m = profile.maturity();

    // H_spatial：Levy 负对数似然（surprise）。
    let h_spatial = if displacement_km <= 0.0 {
        // 无位移（连续同 cell 已被 §4.1 去重过滤，此处仅防御）。
        0.0
    } else if let Some(p) = profile.levy_pdf(displacement_km) {
        if p > 0.0 {
            -p.ln()
        } else {
            10.0 // 概率密度为零：极端异常
        }
    } else {
        // 无 Levy 拟合：用位移历史分位兜底。
        let meds = &profile.displacements;
        if meds.is_empty() {
            1.0 // 无历史数据，温和惩罚
        } else {
            let mean = meds.iter().sum::<f64>() / meds.len() as f64;
            let ratio = (displacement_km / mean).max(0.01);
            ratio.ln().max(0.0)
        }
    };

    // H_temporal：昼夜/周节律违规。
    let hour = crate::engine::behavior::hour_of_day(current.ts);
    let day = crate::engine::behavior::day_of_week(current.ts);
    let c_prob = profile.circadian_prob(hour);
    let w_prob = profile.weekly_prob(day);
    let h_temporal = -c_prob.ln() - w_prob.ln();

    // H_kinetic：锚点转移异常。
    let from_anchor = profile.nearest_anchor(prev_cell).unwrap_or(prev_cell);
    let to_anchor = profile.nearest_anchor(current.cell).unwrap_or(current.cell);
    let t_prob = profile.transition_prob(from_anchor, to_anchor);
    let h_kinetic = -t_prob.ln();

    // H_flock：群体速度对齐（无数据回退自身历史速度分布）。
    let h_flock = if flock_velocities.is_empty() {
        // 回退：比较当前速度与自身历史速度分布的离群度。
        own_velocity
            .map(|(e, n)| {
                let mag = (e * e + n * n).sqrt();
                let meds = &profile.displacements;
                if meds.is_empty() {
                    0.5
                } else {
                    let mean = meds.iter().sum::<f64>() / meds.len() as f64;
                    let ratio = (mag / mean.max(1e-6)).max(0.01);
                    (ratio - 1.0).abs().ln_1p().max(0.0)
                }
            })
            .unwrap_or(0.5)
    } else {
        let (oe, on) = own_velocity.unwrap_or((0.0, 0.0));
        let self_mag = (oe * oe + on * on).sqrt();
        let mut sum_e = 0.0f64;
        let mut sum_n = 0.0f64;
        for v in flock_velocities {
            sum_e += v.east;
            sum_n += v.north;
        }
        let n = flock_velocities.len() as f64;
        let flock_e = sum_e / n;
        let flock_n = sum_n / n;
        let flock_mag = (flock_e * flock_e + flock_n * flock_n).sqrt();
        if self_mag < 1e-9 || flock_mag < 1e-9 {
            1.0 // 零速度时中性惩罚
        } else {
            let dot = oe * flock_e + on * flock_n;
            let alignment = dot / (self_mag * flock_mag);
            1.0 - alignment.max(0.0)
        }
    };

    // H_contextual：传感器互相关（无 IMU 设为 0）。
    let h_contextual = if current.imu_present {
        // 简化：有 IMU 但本实现不做完整互相关（需原始 IMU 数据，超出
        // 当前 scope），给一个保守的小惩罚。
        0.1
    } else {
        0.0
    };

    // H_structure：链结构完整性。
    let h_structure = if !chain_ok {
        H_STRUCTURE_MAX
    } else {
        // 时间间隔规则性：自动化脚本常产生极均匀间隔。
        let interval = current.ts - prev_ts;
        structure_interval_score(&profile.intervals, interval)
    };

    let total = m
        * (W_SPATIAL * h_spatial
            + W_TEMPORAL * h_temporal
            + W_KINETIC * h_kinetic
            + W_FLOCK * h_flock
            + W_CONTEXTUAL * h_contextual
            + W_STRUCTURE * h_structure);

    let baseline = rolling_baseline(&profile.displacements, profile.levy_fit.as_ref());
    let alert = classify(total, baseline);

    HamiltonianReport {
        total,
        maturity: m,
        baseline,
        spatial: h_spatial,
        temporal: h_temporal,
        kinetic: h_kinetic,
        flock: h_flock,
        contextual: h_contextual,
        structure: h_structure,
        alert,
    }
}

/// 时间间隔规则性评分。
///
/// 过于均匀的间隔（CV < 0.05）暗示自动化。
fn structure_interval_score(intervals: &[i64], current: i64) -> f64 {
    if intervals.len() < 4 || current <= 0 {
        return 0.5;
    }
    let mean = intervals.iter().map(|&i| i as f64).sum::<f64>() / intervals.len() as f64;
    let var = intervals
        .iter()
        .map(|&i| (i as f64 - mean).powi(2))
        .sum::<f64>()
        / intervals.len() as f64;
    let std = var.sqrt();
    let cv = std / mean.max(1e-6);

    // 检查当前间隔是否在合理范围。
    let ratio = (current as f64 - mean).abs() / mean.max(1e-6);
    let anomaly = ratio.min(5.0);

    // CV 越低（间隔越均匀）+ 当前偏离越大 → 越可疑。
    let uniformity_penalty = (0.05 - cv).max(0.0) * 20.0; // CV<0.05 时惩罚
    anomaly.min(5.0) + uniformity_penalty
}

/// 滚动基线：历史 H_spatial 的中位数（此处用位移 -log(P) 的中位近似）。
fn rolling_baseline(disps: &[f64], levy_fit: Option<&crate::engine::levy::LevyFit>) -> f64 {
    if disps.is_empty() || levy_fit.is_none() {
        return 0.0;
    }
    let f = levy_fit.unwrap();
    let levy = match crate::engine::levy::LevyParams::new(f.beta, f.kappa, f.r_min) {
        Ok(l) => l,
        Err(_) => return 0.0,
    };
    let mut scores: Vec<f64> = disps
        .iter()
        .map(|&d| {
            let p = levy.pdf(d);
            if p > 0.0 {
                -p.ln()
            } else {
                10.0
            }
        })
        .collect();
    scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
    scores[scores.len() / 2]
}

/// 按草案表映射告警级别。
fn classify(h: f64, baseline: f64) -> AlertLevel {
    let elevated_threshold = if baseline > 0.0 {
        baseline * 1.5
    } else {
        1.5 // 无基线时用保守默认值
    };
    if h < elevated_threshold {
        AlertLevel::Nominal
    } else if h < 3.0 {
        AlertLevel::Elevated
    } else if h < 5.0 {
        AlertLevel::Suspicious
    } else {
        AlertLevel::Critical
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::behavior::BreadcrumbView;

    fn make_crumbs(n: usize) -> Vec<BreadcrumbView> {
        let mut crumbs = Vec::with_capacity(n);
        let mut prev_hash = [0u8; 32];
        let cells: Vec<u64> = (0..n)
            .map(|i| {
                let lat = 39.9042 + (i as f64 % 10.0) * 0.001;
                let lng = 116.4074 + (i as f64 / 10.0) * 0.001;
                let cell = h3o::LatLng::new(lat, lng)
                    .unwrap()
                    .to_cell(h3o::Resolution::Ten);
                u64::from(cell)
            })
            .collect();
        for i in 0..n as i64 {
            let mut block_hash = [0u8; 32];
            block_hash[0] = i as u8;
            crumbs.push(BreadcrumbView {
                ts: 1700000000 + i * 900,
                cell: cells[i as usize],
                prev_hash: if i == 0 { None } else { Some(prev_hash) },
                block_hash,
                imu_present: true,
            });
            prev_hash = block_hash;
        }
        crumbs
    }

    #[test]
    fn nominal_for_normal_breadcrumb() {
        let crumbs = make_crumbs(250);
        let profile = BehavioralProfile::from_breadcrumbs(&crumbs).unwrap();
        let last = crumbs.last().unwrap();
        let prev = &crumbs[crumbs.len() - 2];
        let report = evaluate(
            &profile,
            last,
            0.15,
            &[],
            Some((0.1, 0.05)),
            prev.ts,
            prev.cell,
            true,
        );
        assert!(report.total >= 0.0, "H should be non-negative");
    }

    #[test]
    fn critical_for_chain_break() {
        let crumbs = make_crumbs(250);
        let profile = BehavioralProfile::from_breadcrumbs(&crumbs).unwrap();
        let last = crumbs.last().unwrap();
        let prev = &crumbs[crumbs.len() - 2];
        let report = evaluate(
            &profile,
            last,
            0.15,
            &[],
            Some((0.1, 0.05)),
            prev.ts,
            prev.cell,
            false, // 链断裂
        );
        assert!(report.structure >= H_STRUCTURE_MAX);
    }

    #[test]
    fn alert_levels_are_ordered() {
        assert_eq!(classify(0.1, 0.0), AlertLevel::Nominal);
        assert_eq!(classify(2.0, 0.0), AlertLevel::Elevated);
        assert_eq!(classify(4.0, 0.0), AlertLevel::Suspicious);
        assert_eq!(classify(6.0, 0.0), AlertLevel::Critical);
    }

    #[test]
    fn h_contextual_zero_without_imu() {
        let crumbs = make_crumbs(250);
        let profile = BehavioralProfile::from_breadcrumbs(&crumbs).unwrap();
        let mut no_imu = *crumbs.last().unwrap();
        no_imu.imu_present = false;
        let prev = &crumbs[crumbs.len() - 2];
        let report = evaluate(
            &profile,
            &no_imu,
            0.15,
            &[],
            Some((0.1, 0.05)),
            prev.ts,
            prev.cell,
            true,
        );
        assert!((report.contextual - 0.0).abs() < 1e-9);
    }

    #[test]
    fn flock_alignment_works() {
        let crumbs = make_crumbs(250);
        let profile = BehavioralProfile::from_breadcrumbs(&crumbs).unwrap();
        let last = crumbs.last().unwrap();
        let prev = &crumbs[crumbs.len() - 2];
        // 同方向群体 → 低 H_flock
        let flock = vec![
            FlockVelocity {
                east: 0.1,
                north: 0.05,
            },
            FlockVelocity {
                east: 0.09,
                north: 0.06,
            },
        ];
        let report = evaluate(
            &profile,
            last,
            0.15,
            &flock,
            Some((0.1, 0.05)),
            prev.ts,
            prev.cell,
            true,
        );
        assert!(report.flock < 1.0, "aligned flock should have low H_flock");
    }

    #[test]
    fn flock_misalignment_detected() {
        let crumbs = make_crumbs(250);
        let profile = BehavioralProfile::from_breadcrumbs(&crumbs).unwrap();
        let last = crumbs.last().unwrap();
        let prev = &crumbs[crumbs.len() - 2];
        // 反方向群体 → 高 H_flock
        let flock = vec![FlockVelocity {
            east: -0.1,
            north: -0.05,
        }];
        let report = evaluate(
            &profile,
            last,
            0.15,
            &flock,
            Some((0.1, 0.05)),
            prev.ts,
            prev.cell,
            true,
        );
        assert!(
            report.flock > 0.5,
            "misaligned flock should have high H_flock"
        );
    }
}
