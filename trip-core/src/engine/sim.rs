//! Monte Carlo 轨迹仿真支持（draft-04 §7.3.3 Numerical Validation）。
//!
//! 把连续的截断 Levy 步进轨迹映射为 H3 cell 序列：随机航向 + Levy 步长
//! 的 great-circle 目的地计算 → `LatLng::to_cell` 量化 → 按 §4.1 去重
//! （连续同 cell 不记录）。输出的 cell 序列可直接喂给
//! [`crate::engine::psd::displacements_from_cells`]。

use h3o::{LatLng, Resolution};
use rand::Rng;

use crate::engine::levy::LevyParams;

/// 地球平均半径（km），great-circle 目的地计算使用。
const EARTH_RADIUS_KM: f64 = 6371.0088;
const TAU: f64 = core::f64::consts::TAU;

/// 仿真配置。
#[derive(Debug, Clone, Copy)]
pub struct SimConfig {
    /// 起点纬度（度）。
    pub start_lat_deg: f64,
    /// 起点经度（度）。
    pub start_lng_deg: f64,
    /// 量化分辨率。
    pub resolution: Resolution,
    /// Levy 步长下界（km），应与量化尺度匹配（res10 ≈ 0.1 km）。
    pub r_min_km: f64,
    /// 原始步数上限（去重后不足所需位移数则提前结束，避免死循环）。
    pub raw_step_cap: u32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            start_lat_deg: 39.9042,
            start_lng_deg: 116.4074, // 北京
            resolution: Resolution::Ten,
            r_min_km: 0.1,
            raw_step_cap: 5000,
        }
    }
}

/// 一次仿真产出。
#[derive(Debug, Clone)]
pub struct SimulatedPath {
    /// 去重后的 H3 cell（u64 编码），长度 = 位移数 + 1。
    pub cells: Vec<u64>,
    /// 消耗的原始步数。
    pub raw_steps: u32,
    /// 因连续同 cell 被丢弃的步数。
    pub duplicates_dropped: u32,
}

/// 出行（trip）结构化 Levy 轨迹参数（GeoYuan 扩展）。
///
/// 草案 §7.3.3 的字面生成器（i.i.d. 截断 Levy 步进）产生的位移序列谱是
/// 平坦的（α≈0），无法满足桥关系 g∈[0.3,0.7]。实证还否证了两类"补救"：
/// 线性/对数域活动包络调制步长幅度，都会被重尾 Levy 噪声自身的数量级
/// 涨落淹没（低频信号功率 ∝ E[L]²，噪声底线 ∝ E[L²]），且 §4.1 去重会
/// 把低活动期从记录序列中整体抹除（幅度门控）——谱仍平坦（实测 α≈0）。
///
/// 1/f 记忆的正确来源是**出行结构**（Barabási 人类动力学的时间爆发）：
/// 一次出行 = 先决策走一段 Levy 分布的距离 R，再用 n 个幅度相近的子步
/// 完成（通勤、逛街的连续移动）。相邻记录位移值因此直接相关（≈R/n），
/// 相关信号不受均值-方差比压制，且小出行整段被去重吸收、出行内部的
/// holding 结构在记录序列中保留。重尾边际保留（子步 ∝ R，尾部指数仍
/// 是 β），引擎 MLE 的 β̂ 对其有效。
///
/// `sub-step = (R / n) · exp(σ·N(0,1))`，n ~ U{1..max_substeps}
///
/// α 随 max_substeps 升高、随 jitter_sigma 降低；默认值经
/// examples/monte_carlo --calibrate 校准使中位 α ≈ 0.5–0.6（桥中心
/// g ≈ 0.44）。
#[derive(Debug, Clone, Copy)]
pub struct TripConfig {
    /// 每次出行的最大子步数；子步数 n ~ U{1..=max_substeps}。
    pub max_substeps: usize,
    /// 子步幅度对数抖动 σ。
    pub jitter_sigma: f64,
    /// 子步航向抖动（弧度，标准差）。
    pub heading_jitter_rad: f64,
}

impl Default for TripConfig {
    fn default() -> Self {
        Self {
            max_substeps: 16,
            jitter_sigma: 0.50,
            heading_jitter_rad: 0.35,
        }
    }
}

/// 生成出行结构化的截断 Levy 轨迹（GeoYuan 参考生成器）。
///
/// 空间机制与 [`levy_path`] 相同（great-circle + res10 量化 + §4.1 去重），
/// 仅把"每步独立抽 Levy"换成"每 trip 抽 Levy 距离、按子步走完"。
pub fn trip_walk_path<R: Rng>(
    rng: &mut R,
    beta: f64,
    kappa: f64,
    needed_displacements: usize,
    cfg: &SimConfig,
    trip: &TripConfig,
) -> SimulatedPath {
    let levy = LevyParams::new(beta, kappa, cfg.r_min_km).expect("valid levy params");
    let (mut lat, mut lng) = (
        cfg.start_lat_deg.to_radians(),
        cfg.start_lng_deg.to_radians(),
    );
    let first = LatLng::new(lat.to_degrees(), lng.to_degrees())
        .expect("start in range")
        .to_cell(cfg.resolution);
    let mut cells = vec![u64::from(first)];
    let mut raw = 0u32;
    let mut dropped = 0u32;

    while cells.len() < needed_displacements + 1 && raw < cfg.raw_step_cap {
        let r_trip = levy.sample(rng);
        let n = rng.gen_range(1..=trip.max_substeps);
        let mut bearing = rng.gen_range(0.0..TAU);
        for _ in 0..n {
            if cells.len() > needed_displacements || raw >= cfg.raw_step_cap {
                break;
            }
            raw += 1;
            let r = (r_trip / n as f64) * (trip.jitter_sigma * normal(rng)).exp();
            bearing += trip.heading_jitter_rad * normal(rng);
            let (lat2, lng2) = destination(lat, lng, bearing, r);
            let cell = LatLng::new(lat2.to_degrees(), lng2.to_degrees())
                .expect("destination in range")
                .to_cell(cfg.resolution);
            let raw_cell = u64::from(cell);
            if raw_cell != *cells.last().expect("non-empty") {
                cells.push(raw_cell);
                lat = lat2;
                lng = lng2;
            } else {
                dropped += 1;
            }
        }
    }

    SimulatedPath {
        cells,
        raw_steps: raw,
        duplicates_dropped: dropped,
    }
}

/// 生成一条截断 Levy 随机航向轨迹并量化为 cell 序列。
///
/// 持续步进直到收集到 `needed_displacements + 1` 个去重 cell，
/// 或达到 `raw_step_cap`。
pub fn levy_path<R: Rng>(
    rng: &mut R,
    beta: f64,
    kappa: f64,
    needed_displacements: usize,
    cfg: &SimConfig,
) -> SimulatedPath {
    let levy = LevyParams::new(beta, kappa, cfg.r_min_km).expect("valid levy params");
    let (mut lat, mut lng) = (
        cfg.start_lat_deg.to_radians(),
        cfg.start_lng_deg.to_radians(),
    );
    let first = LatLng::new(lat.to_degrees(), lng.to_degrees())
        .expect("start in range")
        .to_cell(cfg.resolution);
    let mut cells = vec![u64::from(first)];
    let mut raw = 0u32;
    let mut dropped = 0u32;

    while cells.len() < needed_displacements + 1 && raw < cfg.raw_step_cap {
        raw += 1;
        let r = levy.sample(rng);
        let bearing = rng.gen_range(0.0..TAU);
        let (lat2, lng2) = destination(lat, lng, bearing, r);
        let cell = LatLng::new(lat2.to_degrees(), lng2.to_degrees())
            .expect("destination in range")
            .to_cell(cfg.resolution);
        let raw_cell = u64::from(cell);
        if raw_cell != *cells.last().expect("non-empty") {
            cells.push(raw_cell);
            lat = lat2;
            lng = lng2;
        } else {
            dropped += 1;
            // 停留原地：位置不前进（与真实静止一致）。
        }
    }

    SimulatedPath {
        cells,
        raw_steps: raw,
        duplicates_dropped: dropped,
    }
}

/// great-circle 目的地：从 (lat, lng)（弧度）沿方位角 `bearing` 走 `dist_km`。
/// 公开供黄金向量生成器复现同一坐标管线。
pub fn destination(lat: f64, lng: f64, bearing: f64, dist_km: f64) -> (f64, f64) {
    let delta = dist_km / EARTH_RADIUS_KM;
    let sin_lat2 = lat.sin() * delta.cos() + lat.cos() * delta.sin() * bearing.cos();
    let lat2 = sin_lat2.asin();
    let lng2 =
        lng + (bearing.sin() * delta.sin() * lat.cos()).atan2(delta.cos() - lat.sin() * sin_lat2);
    // 归一化到 (-π, π]
    let lng2 = (lng2 + core::f64::consts::PI).rem_euclid(TAU) - core::f64::consts::PI;
    (lat2, lng2)
}

/// Box-Muller 标准正态（两个均匀随机数 → 一个正态样本）。
pub fn normal<R: Rng>(rng: &mut R) -> f64 {
    let u1: f64 = rng.gen();
    let u2: f64 = rng.gen();
    if u1 <= f64::MIN_POSITIVE {
        return 0.0;
    }
    (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn levy_path_produces_enough_cells() {
        let mut rng = StdRng::seed_from_u64(99);
        let cfg = SimConfig::default();
        let path = levy_path(&mut rng, 1.75, 5.0, 256, &cfg);
        assert_eq!(path.cells.len(), 257);
        // 无连续重复（§4.1）
        for w in path.cells.windows(2) {
            assert_ne!(w[0], w[1]);
        }
        assert!(path.duplicates_dropped > 0, "small steps should be deduped");
    }

    #[test]
    fn destination_moves_expected_distance() {
        let (lat, lng) = (39.9042_f64.to_radians(), 116.4074_f64.to_radians());
        let (lat2, lng2) = destination(lat, lng, 0.0, 1.0); // 向正北 1 km
        let dlat = (lat2 - lat).to_degrees();
        // 平均地球半径 6371.0088 km 下每度弧长
        let deg_per_km = 180.0 / (core::f64::consts::PI * EARTH_RADIUS_KM);
        assert!((dlat - deg_per_km).abs() < 1e-9, "dlat = {dlat} deg");
        assert!((lng2 - lng).abs() < 1e-9);
    }
}
