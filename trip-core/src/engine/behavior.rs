//! 行为画像（draft-04 §8 Mobility Profiling）。
//!
//! Verifier 侧为每个身份维护的学习画像，从面包屑链构建，供 Hamiltonian
//! 各分量查询：
//!
//! - **锚点 cell**：≥5 条面包屑的 H3 cell（§8 Anchor cells）；
//! - **Markov 转移矩阵**：锚点间转移计数/概率（§8 Trajectory Predictability）；
//! - **昼夜 / 周节律直方图**：24 小时 + 7 天活动概率（§8 Circadian Profiles）；
//! - **Levy 画像**：历史位移 MLE 拟合的 (β̂, κ̂, r_min)，供 H_spatial 查询。
//!
//! 本模块只做**确定性统计学习**（无外部依赖、无随机性），同一面包屑序列
//! 在任何语言中重建画像应逐位一致。

use std::collections::HashMap;

use h3o::{CellIndex, LatLng};

use crate::engine::levy::{fit, LevyFit};
use crate::engine::psd::displacements_from_cells;
use crate::error::{Result, TripError};

/// 锚点判定阈值：cell 累计面包屑数 ≥ 此值即为锚点（§8）。
pub const ANCHOR_MIN_BREADCRUMBS: usize = 5;
/// Markov 转移矩阵的 ε 地板（防止 -log(0)）。
pub const MARKOV_EPSILON: f64 = 0.001;

/// 从 Unix 秒取 UTC 小时 (0..24)。
pub fn hour_of_day(ts: i64) -> usize {
    ((ts.rem_euclid(86400) / 3600) as usize) % 24
}

/// 从 Unix 秒取 UTC 星期几 (0=周日, 1=周一, …, 6=周六)。
/// 1970-01-01 是星期四 (4)，故 (days + 4) % 7。
pub fn day_of_week(ts: i64) -> usize {
    let days = ts.div_euclid(86400);
    ((days + 4) as usize) % 7
}

/// 单条面包屑的最小结构化视图（足够构建画像与评估 Hamiltonian）。
#[derive(Debug, Clone, Copy)]
pub struct BreadcrumbView {
    /// Unix 秒时间戳。
    pub ts: i64,
    /// H3 cell（u64 编码）。
    pub cell: u64,
    /// 前一块哈希（32 字节），用于链完整性检查。
    pub prev_hash: Option<[u8; 32]>,
    /// 当前块哈希（32 字节）。
    pub block_hash: [u8; 32],
    /// meta flags 的 imu_present 位。
    pub imu_present: bool,
}

/// H3 cell → 该 cell 的面包屑计数。
pub type CellCounts = HashMap<u64, usize>;

/// Markov 转移矩阵：T[from][to] = count，归一化后 P(from→to)。
pub type TransitionMatrix = HashMap<u64, HashMap<u64, usize>>;

/// 昼夜直方图（24 bin）。
pub type CircadianProfile = [f64; 24];

/// 周直方图（7 bin）。
pub type WeeklyProfile = [f64; 7];

/// 身份行为画像（draft-04 §8）。
#[derive(Debug, Clone)]
pub struct BehavioralProfile {
    /// 面包屑总数。
    pub breadcrumb_count: usize,
    /// unique cell 数。
    pub unique_cells: usize,
    /// 锚点 cell 集合（≥5 条面包屑的 cell）。
    pub anchors: Vec<u64>,
    /// Markov 转移计数矩阵。
    pub transitions: TransitionMatrix,
    /// 昼夜活动概率（24 bin，归一化 sum=1）。
    pub circadian: CircadianProfile,
    /// 周活动概率（7 bin，归一化 sum=1）。
    pub weekly: WeeklyProfile,
    /// Levy 拟合参数（位移 MLE）。
    pub levy_fit: Option<LevyFit>,
    /// 历史位移序列（km），供 H_flock 回退使用。
    pub displacements: Vec<f64>,
    /// 首条面包屑时间戳（Unix 秒），用于 trust 的 days_since_first。
    pub first_ts: i64,
    /// 历史时间间隔序列（秒），供 H_structure 的间隔规则性分析。
    pub intervals: Vec<i64>,
}

impl BehavioralProfile {
    /// 从面包屑视图序列构建画像。
    ///
    /// 面包屑须按 index 升序、且至少 1 条；不足 8 条时跳过 Levy 拟合
    ///（`levy_fit` 返回 None，H_spatial 用历史均值兜底）。
    pub fn from_breadcrumbs(crumbs: &[BreadcrumbView]) -> Result<Self> {
        if crumbs.is_empty() {
            return Err(TripError::LevyFit("empty breadcrumb chain".into()));
        }

        let mut cell_counts: CellCounts = HashMap::new();
        let mut transitions: TransitionMatrix = HashMap::new();
        let mut circadian_raw = [0u32; 24];
        let mut weekly_raw = [0u32; 7];
        let mut cells: Vec<u64> = Vec::with_capacity(crumbs.len());
        let mut intervals: Vec<i64> = Vec::with_capacity(crumbs.len().saturating_sub(1));
        let first_ts = crumbs[0].ts;

        for (i, c) in crumbs.iter().enumerate() {
            *cell_counts.entry(c.cell).or_default() += 1;
            circadian_raw[hour_of_day(c.ts)] += 1;
            weekly_raw[day_of_week(c.ts)] += 1;
            cells.push(c.cell);
            if i > 0 {
                let prev = &crumbs[i - 1];
                let dt = c.ts - prev.ts;
                intervals.push(dt);
                // Markov 转移：用 cell u64 直接做 key（后续再按锚点过滤）。
                *transitions
                    .entry(prev.cell)
                    .or_default()
                    .entry(c.cell)
                    .or_default() += 1;
            }
        }

        let unique_cells = cell_counts.len();

        // 锚点：≥5 条面包屑的 cell，按 cell 值升序排序（确定性）。
        let mut anchors: Vec<u64> = cell_counts
            .iter()
            .filter(|(_, &cnt)| cnt >= ANCHOR_MIN_BREADCRUMBS)
            .map(|(&cell, _)| cell)
            .collect();
        anchors.sort_unstable();

        // 锚点间 Markov 矩阵：只保留 from/to 都是锚点的转移。
        let anchor_set: std::collections::HashSet<u64> = anchors.iter().copied().collect();
        let transitions: TransitionMatrix = transitions
            .iter()
            .filter(|(from, _)| anchor_set.contains(from))
            .map(|(from, tos)| {
                let filtered: HashMap<u64, usize> = tos
                    .iter()
                    .filter(|(to, _)| anchor_set.contains(to))
                    .map(|(to, cnt)| (*to, *cnt))
                    .collect();
                (*from, filtered)
            })
            .collect();

        // 归一化昼夜/周直方图。
        let circadian = normalize_hist(&circadian_raw);
        let weekly = normalize_hist(&weekly_raw);

        // 位移序列 + Levy 拟合（≥8 条位移可用）。
        // 先去重连续相同 cell（§4.1 已保证链中无连续重复，此处防御）。
        let dedup_cells: Vec<u64> = {
            let mut out = Vec::with_capacity(cells.len());
            for &c in &cells {
                if out.last() != Some(&c) {
                    out.push(c);
                }
            }
            out
        };
        let displacements = if dedup_cells.len() >= 2 {
            displacements_from_cells(&dedup_cells)?
        } else {
            Vec::new()
        };
        let levy_fit = if displacements.len() >= 8 {
            Some(fit(&displacements, None)?)
        } else {
            None
        };

        Ok(Self {
            breadcrumb_count: crumbs.len(),
            unique_cells,
            anchors,
            transitions,
            circadian,
            weekly,
            levy_fit,
            displacements,
            first_ts,
            intervals,
        })
    }

    /// 给定位移 Δr，查询 Levy 概率密度 P(Δr)。
    ///
    /// 无拟合参数时返回 None（调用方应用历史均值兜底）。
    pub fn levy_pdf(&self, delta_r: f64) -> Option<f64> {
        let f = self.levy_fit?;
        let levy = crate::engine::levy::LevyParams::new(f.beta, f.kappa, f.r_min).ok()?;
        Some(levy.pdf(delta_r))
    }

    /// Markov 转移概率 P(from → to)，找不到返回 ε。
    pub fn transition_prob(&self, from: u64, to: u64) -> f64 {
        let tos = match self.transitions.get(&from) {
            Some(m) => m,
            None => return MARKOV_EPSILON,
        };
        let total: usize = tos.values().sum();
        if total == 0 {
            return MARKOV_EPSILON;
        }
        let cnt = tos.get(&to).copied().unwrap_or(0);
        let p = cnt as f64 / total as f64;
        if p < MARKOV_EPSILON {
            MARKOV_EPSILON
        } else {
            p
        }
    }

    /// 昼夜活动概率 C[hour]，找不到返回 ε。
    pub fn circadian_prob(&self, hour: usize) -> f64 {
        if hour >= 24 {
            return MARKOV_EPSILON;
        }
        let p = self.circadian[hour];
        if p < MARKOV_EPSILON {
            MARKOV_EPSILON
        } else {
            p
        }
    }

    /// 周活动概率 W[day]，找不到返回 ε。
    pub fn weekly_prob(&self, day: usize) -> f64 {
        if day >= 7 {
            return MARKOV_EPSILON;
        }
        let p = self.weekly[day];
        if p < MARKOV_EPSILON {
            MARKOV_EPSILON
        } else {
            p
        }
    }

    /// 成熟度 m = min(breadcrumb_count / 200, 1.0)。
    pub fn maturity(&self) -> f64 {
        (self.breadcrumb_count as f64 / 200.0).min(1.0)
    }

    /// 找最近的锚点 cell（给定任意 cell，找地理距离最近的锚点）。
    pub fn nearest_anchor(&self, cell: u64) -> Option<u64> {
        if self.anchors.is_empty() {
            return None;
        }
        let target = CellIndex::try_from(cell).ok()?;
        let target_ll = LatLng::from(target);
        let mut best = (f64::INFINITY, self.anchors[0]);
        for &a in &self.anchors {
            let a_idx = match CellIndex::try_from(a) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let a_ll = LatLng::from(a_idx);
            let d = haversine_deg(target_ll.lat(), target_ll.lng(), a_ll.lat(), a_ll.lng());
            if d < best.0 {
                best = (d, a);
            }
        }
        Some(best.1)
    }
}

/// 归一化直方图为概率分布（sum=1）。
fn normalize_hist<const N: usize>(raw: &[u32; N]) -> [f64; N] {
    let total: u32 = raw.iter().sum();
    if total == 0 {
        return [1.0 / N as f64; N];
    }
    let mut out = [0.0f64; N];
    for i in 0..N {
        out[i] = raw[i] as f64 / total as f64;
    }
    out
}

/// haversine 距离（度输入，km 输出）。
fn haversine_deg(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R_KM: f64 = 6371.0088;
    let la1 = lat1.to_radians();
    let la2 = lat2.to_radians();
    let dla = (lat2 - lat1).to_radians();
    let dln = (lng2 - lng1).to_radians();
    let a = (dla / 2.0).sin().powi(2) + la1.cos() * la2.cos() * (dln / 2.0).sin().powi(2);
    R_KM * 2.0 * a.sqrt().asin()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_crumbs(n: usize) -> Vec<BreadcrumbView> {
        let mut crumbs = Vec::with_capacity(n);
        let mut prev_hash = [0u8; 32];
        // 用 LatLng 生成合法的 res10 cell 序列（北京附近微小偏移）。
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
                ts: 1700000000 + i * 900, // 15 分钟间隔
                cell: cells[i as usize],
                prev_hash: if i == 0 { None } else { Some(prev_hash) },
                block_hash,
                imu_present: i % 2 == 0,
            });
            prev_hash = block_hash;
        }
        crumbs
    }

    #[test]
    fn profile_builds_from_breadcrumbs() {
        let crumbs = make_crumbs(30);
        let p = BehavioralProfile::from_breadcrumbs(&crumbs).unwrap();
        assert_eq!(p.breadcrumb_count, 30);
        assert!(p.unique_cells > 0);
        assert!(!p.circadian.is_empty());
        assert!(!p.weekly.is_empty());
    }

    #[test]
    fn circadian_prob_is_valid() {
        let crumbs = make_crumbs(30);
        let p = BehavioralProfile::from_breadcrumbs(&crumbs).unwrap();
        let h = hour_of_day(crumbs[0].ts);
        assert!(p.circadian_prob(h) >= MARKOV_EPSILON);
        assert!(p.circadian_prob(23) >= MARKOV_EPSILON);
    }

    #[test]
    fn maturity_caps_at_one() {
        let crumbs = make_crumbs(300);
        let p = BehavioralProfile::from_breadcrumbs(&crumbs).unwrap();
        assert_eq!(p.maturity(), 1.0);
    }

    #[test]
    fn maturity_below_one_for_small_profiles() {
        let crumbs = make_crumbs(100);
        let p = BehavioralProfile::from_breadcrumbs(&crumbs).unwrap();
        assert!((p.maturity() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn day_of_week_is_correct() {
        // 1970-01-01 是星期四，对应 day=4
        assert_eq!(day_of_week(0), 4);
        // 1970-01-04 是星期日，对应 day=0
        assert_eq!(day_of_week(86400 * 3), 0);
    }

    #[test]
    fn hour_of_day_wraps() {
        assert_eq!(hour_of_day(0), 0); // 00:00 UTC
        assert_eq!(hour_of_day(3600), 1); // 01:00 UTC
        assert_eq!(hour_of_day(86400), 0); // next day 00:00
    }
}
