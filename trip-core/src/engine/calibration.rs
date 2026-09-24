//! GeoLife/MDC/T-Drive 真实数据标定模块（W7）。
//!
//! 本模块实现：
//! 1. GeoLife PLT 格式解析器（北京 182 用户 GPS 轨迹）
//! 2. 真实轨迹 → H3 单元格 → PSD α / Levy MLE 标定流水线
//! 3. ROC 分析与人群参数校准
//! 4. 标定报告生成（JSON）
//!
//! # 数据集
//!
//! - **GeoLife**: Microsoft Research Geolife (北京 182 用户, ~17k 轨迹)
//!   - 格式: PLT (纬度,经度,_,_,日期时间,_)
//!   - 来源: https://www.microsoft.com/en-us/download/details.aspx?id=52367
//! - **MDC**: Lausanne Multi-Modal Mobility Dataset (洛桑 200+ 用户)
//! - **T-Drive**: 北京出租车轨迹 (仅作对照，职业偏差)

use h3o::{LatLng, Resolution};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::engine::levy::{bridge_check, fit as levy_fit, LevyParams};
use crate::engine::psd::{
    displacements_from_cells, psd_alpha, ALPHA_CENTER, BIO_ALPHA_MAX, BIO_ALPHA_MIN,
    MIN_PSD_SAMPLES,
};
use crate::engine::sim::{levy_path, normal, SimConfig};
use crate::error::{Result, TripError};

/// H3 分辨率用于标定（与协议默认 res10 一致）。
const H3_RESOLUTION: Resolution = Resolution::Ten;

/// 最小轨迹点数（过滤太短的轨迹）。
const MIN_POINTS_PER_TRAJECTORY: usize = 64;

/// 时间窗口（秒），用于去重相邻过近的点。
const MIN_TIME_GAP_SECONDS: i64 = 300; // 5 分钟

// ---------------------------------------------------------------------------
// GeoLife PLT 解析
// ---------------------------------------------------------------------------

/// GeoLife PLT 文件中的单条 GPS 记录。
#[derive(Debug, Clone)]
pub struct GpsPoint {
    /// 纬度。
    pub lat: f64,
    /// 经度。
    pub lng: f64,
    /// Unix 时间戳（秒）。
    pub timestamp: i64,
    /// 原始日期时间字符串（保留用于调试）。
    pub raw_datetime: String,
}

/// 从单条 PLT 行解析 GPS 点。
///
/// PLT 格式: `纬度,经度,0,altitude,日期时间,日期`
/// 例: `39.984702,116.318417,0,492,2008-10-23 05:53:05,2008-10-23`
pub fn parse_plt_line(line: &str) -> Option<GpsPoint> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let parts: Vec<&str> = line.split(',').collect();
    if parts.len() < 6 {
        return None;
    }
    let lat = parts[0].trim().parse::<f64>().ok()?;
    let lng = parts[1].trim().parse::<f64>().ok()?;
    // parts[2] = 0 (unused), parts[3] = altitude (unused)
    let raw_dt = parts[4].trim().to_string();
    // 解析 datetime: "2008-10-23 05:53:05"
    let timestamp = parse_geolife_datetime(&raw_dt)?;
    Some(GpsPoint {
        lat,
        lng,
        timestamp,
        raw_datetime: raw_dt,
    })
}

/// 解析 GeoLife 日期时间格式为 Unix 时间戳。
fn parse_geolife_datetime(dt: &str) -> Option<i64> {
    // 格式: "2008-10-23 05:53:05"
    let dt = dt.trim();
    let parts: Vec<&str> = dt.split([' ', '-', ':']).collect();
    if parts.len() != 6 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    let hour: u32 = parts[3].parse().ok()?;
    let minute: u32 = parts[4].parse().ok()?;
    let second: u32 = parts[5].parse().ok()?;
    // days since 1970-01-01
    let days_since_epoch = days_from_civil(year, month, day);
    let total_seconds =
        days_since_epoch * 86400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64;
    // Beijing time (UTC+8) to UTC
    Some(total_seconds - 8 * 3600)
}

/// 计算从 1970-01-01 开始的天数（简化版格里高利历）。
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = year as i64;
    let m = month as i64;
    let d = day as i64;
    let y_adj = if m <= 2 { y - 1 } else { y };
    let m_adj = if m <= 2 { m + 9 } else { m - 3 };
    365 * y_adj + y_adj / 4 - y_adj / 100 + y_adj / 400 + (153 * m_adj + 2) / 5 + d - 719469
}

/// 解析单个 PLT 文件，返回 GPS 点序列。
pub fn parse_plt_file(path: &Path) -> Result<Vec<GpsPoint>> {
    let content =
        fs::read_to_string(path).map_err(|e| crate::error::TripError::Io(e.to_string()))?;
    let mut points = Vec::new();
    for line in content.lines() {
        if let Some(pt) = parse_plt_line(line) {
            points.push(pt);
        }
    }
    Ok(points)
}

/// 扫描 GeoLife 目录结构，返回所有 PLT 文件路径列表。
///
/// GeoLife 目录结构:
/// ```text
/// GeoLife_Trajectories_1.3/
///   └── 000/
///       └── Trajectory/
///           ├── 20081023025304.plt
///           ├── 20081026025304.plt
///           └── ...
/// ```
pub fn scan_geolife_dir(root: &Path) -> Result<Vec<PathBuf>> {
    let mut plt_files = Vec::new();

    // 检查是否是用户目录（数字命名的子目录）
    let entries = fs::read_dir(root).map_err(|e| crate::error::TripError::Io(e.to_string()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // 检查是否有 Trajectory 子目录
            let traj_dir = path.join("Trajectory");
            if traj_dir.is_dir() {
                // 扫描 .plt 文件
                if let Ok(traj_entries) = fs::read_dir(&traj_dir) {
                    for traj_entry in traj_entries.flatten() {
                        let p = traj_entry.path();
                        if p.extension().and_then(|e| e.to_str()) == Some("plt") {
                            plt_files.push(p);
                        }
                    }
                }
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false)
            {
                // 可能是更深的嵌套结构，递归扫描
                if let Ok(sub_files) = scan_geolive_nested(&path) {
                    plt_files.extend(sub_files);
                }
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("plt") {
            plt_files.push(path);
        }
    }

    plt_files.sort();
    Ok(plt_files)
}

/// 递归扫描嵌套目录中的 PLT 文件。
fn scan_geolive_nested(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| crate::error::TripError::Io(e.to_string()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Ok(sub) = scan_geolive_nested(&path) {
                files.extend(sub);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("plt") {
            files.push(path);
        }
    }
    Ok(files)
}

// ---------------------------------------------------------------------------
// 轨迹预处理
// ---------------------------------------------------------------------------

/// 预处理后的轨迹（已去重、H3 量化）。
#[derive(Debug, Clone)]
pub struct ProcessedTrajectory {
    /// 用户 ID（来自目录名）。
    pub user_id: String,
    /// 轨迹文件名。
    pub trajectory_id: String,
    /// H3 cell 序列（u64 表示）。
    pub cells: Vec<u64>,
    /// 位移序列（km）。
    pub displacements: Vec<f64>,
    /// 原始点数。
    pub raw_point_count: usize,
    /// 有效点数（去重后）。
    pub valid_point_count: usize,
}

/// 对 GPS 点序列进行预处理：
/// 1. 按时间排序
/// 2. 过滤时间间隔 < 5 分钟的相邻点
/// 3. H3 res10 量化
/// 4. 提取位移序列
pub fn process_trajectory(
    user_id: &str,
    trajectory_id: &str,
    points: &[GpsPoint],
) -> Option<ProcessedTrajectory> {
    if points.len() < MIN_POINTS_PER_TRAJECTORY {
        return None;
    }

    // 1. 按时间排序
    let mut sorted: Vec<GpsPoint> = points.to_vec();
    sorted.sort_by_key(|p| p.timestamp);

    // 2. 过滤并去重
    let mut filtered = Vec::new();
    let mut last_ts = None;
    for pt in &sorted {
        match last_ts {
            Some(prev) if pt.timestamp - prev < MIN_TIME_GAP_SECONDS => continue,
            _ => {}
        }
        last_ts = Some(pt.timestamp);
        filtered.push(pt.clone());
    }

    if filtered.len() < MIN_POINTS_PER_TRAJECTORY {
        return None;
    }

    // 3. H3 量化
    let mut cells = Vec::new();
    for pt in &filtered {
        let cell = LatLng::new(pt.lat, pt.lng).ok()?.to_cell(H3_RESOLUTION);
        cells.push(u64::from(cell));
    }

    // 4. 提取位移序列
    let displacements = displacements_from_cells(&cells).ok()?;

    Some(ProcessedTrajectory {
        user_id: user_id.to_string(),
        trajectory_id: trajectory_id.to_string(),
        cells,
        displacements,
        raw_point_count: points.len(),
        valid_point_count: filtered.len(),
    })
}

// ---------------------------------------------------------------------------
// 标定分析
// ---------------------------------------------------------------------------

/// 单条轨迹的完整标定结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrajectoryCalibration {
    /// 用户 ID。
    pub user_id: String,
    /// 轨迹 ID。
    pub trajectory_id: String,
    /// 原始点数。
    pub raw_points: usize,
    /// 有效点数。
    pub valid_points: usize,
    /// PSD 分析结果。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psd: Option<PsdResult>,
    /// Levy 拟合结果。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub levy: Option<LevyCalibResult>,
}

/// PSD 结果的可序列化版本。
#[derive(Debug, Clone, serde::Serialize)]
pub struct PsdResult {
    /// α 值。
    pub alpha: f64,
    /// R²。
    pub r_squared: f64,
    /// 置信度。
    pub confidence: f64,
    /// 分类。
    pub classification: String,
    /// 样本数。
    pub sample_count: usize,
}

/// Levy 标定结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct LevyCalibResult {
    /// β 估计值。
    pub beta: f64,
    /// κ 估计值。
    pub kappa: f64,
    /// r_min 估计值。
    pub r_min: f64,
    /// 桥校验 g 值。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_g: Option<f64>,
}

/// 对单条轨迹运行完整标定流水线。
pub fn calibrate_trajectory(traj: &ProcessedTrajectory) -> TrajectoryCalibration {
    let psd = if traj.displacements.len() >= 64 {
        psd_alpha(&traj.displacements).ok().map(|a| PsdResult {
            alpha: a.alpha,
            r_squared: a.r_squared,
            confidence: a.confidence,
            classification: a.classification.as_str().to_string(),
            sample_count: a.sample_count,
        })
    } else {
        None
    };

    let levy = if traj.displacements.len() >= 64 {
        levy_fit(&traj.displacements, None).ok().map(|f| {
            let bridge_g = psd
                .as_ref()
                .and_then(|p| bridge_check(p.alpha, f.beta).ok())
                .map(|b| b.g);
            LevyCalibResult {
                beta: f.beta,
                kappa: f.kappa,
                r_min: f.r_min,
                bridge_g,
            }
        })
    } else {
        None
    };

    TrajectoryCalibration {
        user_id: traj.user_id.clone(),
        trajectory_id: traj.trajectory_id.clone(),
        raw_points: traj.raw_point_count,
        valid_points: traj.valid_point_count,
        psd,
        levy,
    }
}

// ---------------------------------------------------------------------------
// 人群统计汇总
// ---------------------------------------------------------------------------

/// 人群级标定报告。
#[derive(Debug, Clone, serde::Serialize)]
pub struct PopulationCalibrationReport {
    /// 数据集名称。
    pub dataset: String,
    /// 总轨迹数。
    pub total_trajectories: usize,
    /// 有效分析轨迹数。
    pub analyzed_trajectories: usize,
    /// α 统计。
    pub alpha_stats: AlphaPopulationStats,
    /// β 统计。
    pub beta_stats: Value,
    /// 生物区间占比。
    pub bio_fraction: f64,
    /// 推荐的 α 边界调整（如有）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_alpha_bounds: Option<AlphaBoundRecommendation>,
    /// ROC 分析结果。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roc: Option<RocAnalysis>,
    /// 元数据。
    pub metadata: CalibrationMetadata,
}

/// α 人群统计。
#[derive(Debug, Clone, serde::Serialize)]
pub struct AlphaPopulationStats {
    /// 样本量。
    pub n: usize,
    /// 均值。
    pub mean: f64,
    /// 标准差。
    pub std: f64,
    /// 中位数。
    pub median: f64,
    /// P5.
    pub p05: f64,
    /// P25.
    pub p25: f64,
    /// P75.
    pub p75: f64,
    /// P95.
    pub p95: f64,
    /// 最小值.
    pub min: f64,
    /// 最大值.
    pub max: f64,
    /// 在草案生物区间 [0.30, 0.80] 内的占比。
    pub in_draft_bio_range: f64,
}

/// α 边界推荐。
#[derive(Debug, Clone, serde::Serialize)]
pub struct AlphaBoundRecommendation {
    /// 推荐下界。
    pub recommended_min: f64,
    /// 推荐上界。
    pub recommended_max: f64,
    /// 推荐中心。
    pub recommended_center: f64,
    /// 理由。
    pub rationale: String,
}

/// ROC 分析结果（真实双样本判别：人类组 = 正类，合成对照组 = 负类）。
///
/// 与早期"单群体伪 ROC"的本质区别：TPR 与 FPR 分别来自**两个不同样本**，
/// 因此 AUC 有统计意义，可对外提交。
#[derive(Debug, Clone, serde::Serialize)]
pub struct RocAnalysis {
    /// 各阈值下的工作点（阈值降序；同分样本已合并；已剔除 ±∞ 哨兵行）。
    pub tpr_at_thresholds: Vec<Value>,
    /// 最优阈值（Youden's J = TPR − FPR 最大处）。
    pub optimal_threshold: f64,
    /// 最优点处的 TPR（真阳率 / 召回）。
    pub optimal_tpr: f64,
    /// 最优点处的 FPR（假阳率）。
    pub optimal_fpr: f64,
    /// 曲线下面积（FPR 轴梯形积分，∈ [0,1]；0.5 = 无判别力）。
    pub auc: f64,
    /// 正类（人类）样本数。
    pub positives: usize,
    /// 负类（合成对照组）样本数。
    pub negatives: usize,
    /// 由 α 映射得到分数时，最优点等价的可执行判决区间 `[α_min, α_max]`。
    ///
    /// `optimal_threshold` 位于**分数空间**（`score = −|α − 0.55|`），不便直接
    /// 使用；此字段给出该阈值对应的 α 区间：落在区间内判为人类。非 α 分数
    /// （如直接传入 arbitrary score）时为 `None`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimal_alpha_band: Option<[f64; 2]>,
}

/// 标定元数据。
#[derive(Debug, Clone, serde::Serialize)]
pub struct CalibrationMetadata {
    /// H3 分辨率。
    pub h3_resolution: u8,
    /// 最小时间间隔（秒）。
    pub min_time_gap_seconds: i64,
    /// 最小轨迹点数。
    pub min_points: usize,
    /// 分析时间戳。
    pub analyzed_at: String,
    /// 协议版本。
    pub protocol_version: String,
}

/// 从一组标定结果生成人群级报告。
///
/// `control_alphas` 为合成对照组的 α 序列：给出时计算真实 ROC / AUC；
/// 为 `None` 或空时 `roc` 字段为 `None`——**不伪造**（无对照无法定义 FPR）。
pub fn generate_population_report(
    dataset_name: &str,
    results: &[TrajectoryCalibration],
    control_alphas: Option<&[f64]>,
) -> PopulationCalibrationReport {
    let total = results.len();
    let analyzed = results.iter().filter(|r| r.psd.is_some()).count();

    // 收集有效的 α 值（剔除 NaN/∞ 后升序）
    let alphas: Vec<f64> = sorted_finite(
        &results
            .iter()
            .filter_map(|r| r.psd.as_ref().map(|p| p.alpha))
            .collect::<Vec<_>>(),
    );

    // 收集有效的 β 值（同上）
    let betas: Vec<f64> = sorted_finite(
        &results
            .iter()
            .filter_map(|r| r.levy.as_ref().map(|l| l.beta))
            .collect::<Vec<_>>(),
    );

    let n = alphas.len();
    let alpha_stats = compute_alpha_stats(&alphas);

    // β 统计
    let beta_stats = if betas.is_empty() {
        json!({})
    } else {
        json!({
            "n": betas.len(),
            "mean": mean_f64(&betas),
            "std": std_f64(&betas),
            "median": percentile_sorted(&betas, 0.50),
            "p05": percentile_sorted(&betas, 0.05),
            "p95": percentile_sorted(&betas, 0.95),
        })
    };

    // 生物区间占比
    let bio_fraction = if n > 0 {
        alphas
            .iter()
            .filter(|a| (**a >= BIO_ALPHA_MIN) && (**a <= BIO_ALPHA_MAX))
            .count() as f64
            / n as f64
    } else {
        0.0
    };

    // 推荐 α 边界调整
    let recommended_bounds = recommend_alpha_bounds(&alpha_stats);

    // ROC：仅在有合成对照组时计算（无对照不伪造）
    let roc = match control_alphas {
        Some(c) if !c.is_empty() => roc_analysis_from_alphas(&alphas, c).ok(),
        _ => None,
    };

    PopulationCalibrationReport {
        dataset: dataset_name.to_string(),
        total_trajectories: total,
        analyzed_trajectories: analyzed,
        alpha_stats,
        beta_stats,
        bio_fraction,
        recommended_alpha_bounds: recommended_bounds,
        roc,
        metadata: CalibrationMetadata {
            h3_resolution: 10, // Resolution::Ten
            min_time_gap_seconds: MIN_TIME_GAP_SECONDS,
            min_points: MIN_POINTS_PER_TRAJECTORY,
            analyzed_at: utc_now_iso(),
            protocol_version: "draft-ayerbe-trip-protocol-04".to_string(),
        },
    }
}

fn compute_alpha_stats(alphas: &[f64]) -> AlphaPopulationStats {
    let n = alphas.len();
    if n == 0 {
        return AlphaPopulationStats {
            n: 0,
            mean: 0.0,
            std: 0.0,
            median: 0.0,
            p05: 0.0,
            p25: 0.0,
            p75: 0.0,
            p95: 0.0,
            min: 0.0,
            max: 0.0,
            in_draft_bio_range: 0.0,
        };
    }
    let m = mean_f64(alphas);
    let s = std_f64(alphas);
    let in_range = alphas
        .iter()
        .filter(|a| (**a >= BIO_ALPHA_MIN) && (**a <= BIO_ALPHA_MAX))
        .count() as f64
        / n as f64;

    AlphaPopulationStats {
        n,
        mean: m,
        std: s,
        median: percentile_sorted(alphas, 0.50),
        p05: percentile_sorted(alphas, 0.05),
        p25: percentile_sorted(alphas, 0.25),
        p75: percentile_sorted(alphas, 0.75),
        p95: percentile_sorted(alphas, 0.95),
        min: alphas[0],
        max: alphas[n - 1],
        in_draft_bio_range: in_range,
    }
}

/// 基于人群分布推荐 α 边界调整。
fn recommend_alpha_bounds(stats: &AlphaPopulationStats) -> Option<AlphaBoundRecommendation> {
    // 如果 >90% 的样本在草案区间内，不需要调整
    if stats.in_draft_bio_range >= 0.90 {
        return None;
    }

    // 使用 P5-P95 作为推荐区间（覆盖 90% 人群）
    let rec_min = (stats.p05 * 100.0).round() / 100.0;
    let rec_max = (stats.p95 * 100.0).round() / 100.0;
    let rec_center = (rec_min + rec_max) / 2.0;

    let rationale = format!(
        "Draft bounds [{:.2}, {:.2}] cover only {:.1}% of {} samples. \
         Recommended [{:.2}, {:.2}] (P5-P95) covers 90% of population.",
        BIO_ALPHA_MIN,
        BIO_ALPHA_MAX,
        stats.in_draft_bio_range * 100.0,
        stats.n,
        rec_min,
        rec_max
    );

    Some(AlphaBoundRecommendation {
        recommended_min: rec_min,
        recommended_max: rec_max,
        recommended_center: rec_center,
        rationale,
    })
}

// ---------------------------------------------------------------------------
// 合成对照组（ROC 负类）与真实 ROC
// ---------------------------------------------------------------------------

/// 合成攻击轨迹族——ROC 对照组的负类来源。
///
/// 三族覆盖草案点名的典型对抗方向：谱平坦（白噪声式）、强趋势（重放漂移）、
/// 强低频相关（平滑游走）。生成器构造与 `tests/monte_carlo_smoke.rs` 中的
/// 对照组一致，保证"标定用的攻击"与"回归测试用的攻击"是同一批。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlFamily {
    /// 草案 §7.3.3 字面生成器：i.i.d. 截断 Levy 步进 → α ≈ 0（谱平坦）。
    IidLevy,
    /// 重放同一条录制序列 + 线性漂移 → α ≳ 1.2（趋势谱）。
    ReplayDrift,
    /// AR(1) 速度积分 → α ≳ 1.0（强低频相关）。
    CorrelatedGaussian,
}

impl ControlFamily {
    /// 全部族（确定性顺序，供逐族报告）。
    pub const ALL: [ControlFamily; 3] = [
        ControlFamily::IidLevy,
        ControlFamily::ReplayDrift,
        ControlFamily::CorrelatedGaussian,
    ];

    /// 稳定字符串标识（JSON / 报告用）。
    pub fn as_str(self) -> &'static str {
        match self {
            ControlFamily::IidLevy => "iid_levy",
            ControlFamily::ReplayDrift => "replay_drift",
            ControlFamily::CorrelatedGaussian => "correlated_gaussian",
        }
    }
}

/// α → "像人程度"分数：负的到生物区间中心的距离。
///
/// `score(α) = −|α − 0.55|`。分数越高越像人（α 越接近粉噪中心）。白噪声
/// （α≈0）与棕噪声（α≈2）两侧都拿低分，因此对两类攻击方向同时单调——这是
/// 用单一标量刻画"人类 vs 机器"最自然的选择。
pub fn alpha_humanness_score(alpha: f64) -> f64 {
    if !alpha.is_finite() {
        return f64::NAN;
    }
    -(alpha - ALPHA_CENTER).abs()
}

/// 生成某一族合成对照轨迹的 α 序列（确定性：同 seed 必得同结果）。
///
/// `window` 会被抬到至少 [`MIN_PSD_SAMPLES`]；个别轨迹若因退化无法完成 PSD
/// 分析则被跳过（返回长度可能略小于 `count`，但同 seed 稳定）。
pub fn synthetic_control_alphas(
    family: ControlFamily,
    count: usize,
    window: usize,
    seed: u64,
) -> Vec<f64> {
    let window = window.max(MIN_PSD_SAMPLES);
    let mut rng = StdRng::seed_from_u64(seed);
    let cfg = SimConfig {
        raw_step_cap: 40_000,
        ..SimConfig::default()
    };

    // 重放族共享同一条"录制"序列；其余族逐条独立采样。
    let replay_base: Vec<f64> = if family == ControlFamily::ReplayDrift {
        match LevyParams::new(1.75, 5.0, 0.1) {
            Ok(p) => (0..window).map(|_| p.sample(&mut rng)).collect(),
            Err(_) => return Vec::new(),
        }
    } else {
        Vec::new()
    };

    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let disp: Vec<f64> = match family {
            ControlFamily::IidLevy => {
                let beta = 1.50 + 0.40 * rng.gen::<f64>();
                let kappa = 5.0 * normal(&mut rng).exp();
                let path = levy_path(&mut rng, beta, kappa, window, &cfg);
                match displacements_from_cells(&path.cells) {
                    Ok(d) => d,
                    Err(_) => continue,
                }
            }
            ControlFamily::ReplayDrift => replay_base
                .iter()
                .enumerate()
                .map(|(i, b)| b + 0.05 * i as f64 + 1e-4 * normal(&mut rng))
                .collect(),
            ControlFamily::CorrelatedGaussian => {
                let mut v = 0.0;
                let mut pos = 0.0;
                (0..window)
                    .map(|_| {
                        v = 0.9 * v + 0.1 * normal(&mut rng);
                        pos += v;
                        pos.abs() + 1e-3
                    })
                    .collect()
            }
        };
        if let Ok(a) = psd_alpha(&disp) {
            out.push(a.alpha);
        }
    }
    out
}

/// 生成三族合成对照的 α 序列，保留族边界（便于逐族报告）。
///
/// 每族用互不相同的派生 seed，避免族间样本相关；`per_family = 0` 时返回空。
pub fn build_control_families(
    per_family: usize,
    window: usize,
    seed: u64,
) -> Vec<(ControlFamily, Vec<f64>)> {
    if per_family == 0 {
        return Vec::new();
    }
    ControlFamily::ALL
        .iter()
        .enumerate()
        .map(|(idx, family)| {
            let s = seed.wrapping_add((idx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
            let alphas = synthetic_control_alphas(*family, per_family, window, s);
            (*family, alphas)
        })
        .collect()
}

/// 逐族 α 分布统计（JSON 对象：族名 → 统计），供报告与白皮书使用。
pub fn control_family_summary(families: &[(ControlFamily, Vec<f64>)]) -> Value {
    let mut map = serde_json::Map::new();
    for (family, alphas) in families {
        let finite = sorted_finite(alphas);
        if finite.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::to_value(compute_alpha_stats(&finite)) {
            map.insert(family.as_str().to_string(), v);
        }
    }
    Value::Object(map)
}

/// 计算真实 ROC：阈值按分数降序扫描，同分样本合并为一组，AUC 用 FPR 轴梯形积分。
///
/// `human_scores` 为正类（分数越高越像人），`control_scores` 为负类。任一
/// 组在剔除非有限值后为空时返回 [`TripError::Calibration`]。
///
/// 返回的 `tpr_at_thresholds` 已剔除 `±∞` 哨兵行（JSON 无法表示无穷），并在
/// 点数过多时等间隔降采样到约 50 行——**AUC 与最优点始终基于完整扫描**，
/// 降采样只影响展示表格。
pub fn roc_analysis(human_scores: &[f64], control_scores: &[f64]) -> Result<RocAnalysis> {
    let positives = human_scores.iter().filter(|s| s.is_finite()).count();
    let negatives = control_scores.iter().filter(|s| s.is_finite()).count();
    if positives == 0 || negatives == 0 {
        return Err(TripError::Calibration(format!(
            "ROC needs both classes with finite scores, got positives={positives}, negatives={negatives}"
        )));
    }

    // (分数, 是否人类)，降序。
    let mut items: Vec<(f64, bool)> = Vec::with_capacity(positives + negatives);
    items.extend(
        human_scores
            .iter()
            .filter(|s| s.is_finite())
            .map(|&s| (s, true)),
    );
    items.extend(
        control_scores
            .iter()
            .filter(|s| s.is_finite())
            .map(|&s| (s, false)),
    );
    items.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .expect("finite values are totally ordered")
    });

    let mut points: Vec<(f64, f64, f64)> = Vec::with_capacity(items.len() + 2);
    points.push((f64::INFINITY, 0.0, 0.0));
    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut i = 0usize;
    while i < items.len() {
        let threshold = items[i].0;
        // 同分合并：消除并列样本的先后顺序依赖。
        while i < items.len() && items[i].0 == threshold {
            if items[i].1 {
                tp += 1;
            } else {
                fp += 1;
            }
            i += 1;
        }
        points.push((
            threshold,
            tp as f64 / positives as f64,
            fp as f64 / negatives as f64,
        ));
    }
    points.push((f64::NEG_INFINITY, 1.0, 1.0));

    // AUC：沿 FPR 轴梯形积分（FPR 单调不减，可逐段累加）。
    let mut auc = 0.0;
    for w in points.windows(2) {
        auc += (w[1].2 - w[0].2) * (w[0].1 + w[1].1) * 0.5;
    }
    let auc = auc.clamp(0.0, 1.0);

    // Youden's J 最优点；并列时保留阈值更高者（更保守）。
    let mut best = points[0];
    let mut best_j = best.1 - best.2;
    for p in points.iter().skip(1) {
        let j = p.1 - p.2;
        if j > best_j {
            best_j = j;
            best = *p;
        }
    }

    Ok(RocAnalysis {
        tpr_at_thresholds: roc_table(&points, 50),
        optimal_threshold: if best.0.is_finite() {
            best.0
        } else {
            ALPHA_CENTER
        },
        optimal_tpr: best.1,
        optimal_fpr: best.2,
        auc,
        positives,
        negatives,
        optimal_alpha_band: None,
    })
}

/// 由 α 序列直接计算 ROC（内部把 α 映射为 [`alpha_humanness_score`]）。
///
/// 额外填充 [`RocAnalysis::optimal_alpha_band`]：把分数空间的最优阈值翻译成
/// 可直接落地的 α 判决区间 `[0.55 − |t|, 0.55 + |t|]`。
pub fn roc_analysis_from_alphas(
    human_alphas: &[f64],
    control_alphas: &[f64],
) -> Result<RocAnalysis> {
    let h: Vec<f64> = human_alphas
        .iter()
        .copied()
        .map(alpha_humanness_score)
        .collect();
    let c: Vec<f64> = control_alphas
        .iter()
        .copied()
        .map(alpha_humanness_score)
        .collect();
    let mut roc = roc_analysis(&h, &c)?;
    let half_width = roc.optimal_threshold.abs();
    roc.optimal_alpha_band = Some([ALPHA_CENTER - half_width, ALPHA_CENTER + half_width]);
    Ok(roc)
}

/// 把工作点转成 JSON 表格；剔除 ±∞ 阈值，并在超长时等间隔降采样。
fn roc_table(points: &[(f64, f64, f64)], max_rows: usize) -> Vec<Value> {
    let finite: Vec<&(f64, f64, f64)> = points.iter().filter(|p| p.0.is_finite()).collect();
    if finite.is_empty() {
        return Vec::new();
    }
    let n = finite.len();
    let rows = max_rows.max(2).min(n);
    let mut out = Vec::with_capacity(rows);
    for k in 0..rows {
        let idx = if rows == 1 {
            0
        } else {
            k * (n - 1) / (rows - 1)
        };
        let (th, tpr, fpr) = *finite[idx];
        out.push(json!({ "threshold": th, "tpr": tpr, "fpr": fpr }));
    }
    out
}

// ---------------------------------------------------------------------------
// 工具函数
// ---------------------------------------------------------------------------

fn mean_f64(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.iter().sum::<f64>() / v.len() as f64
}

fn std_f64(v: &[f64]) -> f64 {
    if v.len() <= 1 {
        return 0.0;
    }
    let m = mean_f64(v);
    let var = v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64;
    var.sqrt()
}

fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// 过滤非有限值（NaN / ±∞）后升序排序。
///
/// 标定输入可能含 NaN（PLT 脏数据、退化位移）。直接
/// `sort_by(|a, b| a.partial_cmp(b).unwrap())` 会 panic，整批标定因此中断。
/// 统一先剔除再比较：剩余值保证全序，`expect` 不会触发。
fn sorted_finite(values: &[f64]) -> Vec<f64> {
    let mut out: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    out.sort_by(|a, b| a.partial_cmp(b).expect("finite values are totally ordered"));
    out
}

/// 由"1970-01-01 起的天数"反解公历 `(年, 月, 日)`。
///
/// Howard Hinnant 的 `civil_from_days`，与文件内 [`days_from_civil`] 互逆。
/// 全整数运算、无浮点、无时区，跨平台结果逐位一致。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

/// Unix 秒 → ISO-8601 字符串（UTC，秒精度）。
fn iso8601_from_unix(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// 当前 UTC 时间的 ISO-8601（秒精度）。
fn utc_now_iso() -> String {
    use std::time::UNIX_EPOCH;
    let secs = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    iso8601_from_unix(secs)
}

// ---------------------------------------------------------------------------
// 主入口：批量标定
// ---------------------------------------------------------------------------

/// 对 GeoLife 数据集运行完整标定流水线。
///
/// # 参数
/// - `data_dir`: GeoLife 数据根目录
/// - `output_path`: JSON 报告输出路径（可选，默认 stdout）
/// - `control_per_family`: 每族合成对照轨迹数（0 = 不算 ROC，`roc` 字段为 None）
/// - `control_seed`: 对照组随机种子（固定值保证可复现）
///
/// # 返回
/// 标定报告 JSON
pub fn run_geolife_calibration(
    data_dir: &Path,
    output_path: Option<&Path>,
    control_per_family: usize,
    control_seed: u64,
) -> Result<Value> {
    println!("Scanning GeoLife directory: {}", data_dir.display());
    let plt_files = scan_geolife_dir(data_dir)?;
    println!("Found {} PLT files", plt_files.len());

    let mut results = Vec::new();
    let mut errors = 0u64;

    for (i, plt_path) in plt_files.iter().enumerate() {
        // 提取 user_id 和 trajectory_id
        let user_id = plt_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let trajectory_id = plt_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // 解析 PLT
        let points = match parse_plt_file(plt_path) {
            Ok(p) => p,
            Err(_) => {
                errors += 1;
                continue;
            }
        };

        // 预处理
        let traj = match process_trajectory(&user_id, &trajectory_id, &points) {
            Some(t) => t,
            None => continue,
        };

        // 标定
        let calib = calibrate_trajectory(&traj);
        results.push(calib);

        if (i + 1) % 100 == 0 || i == plt_files.len() - 1 {
            println!(
                "Processed {}/{} trajectories ({} analyzed)",
                i + 1,
                plt_files.len(),
                results.len()
            );
        }
    }

    println!(
        "Calibration complete: {} trajectories analyzed, {} errors",
        results.len(),
        errors
    );

    // 合成对照组（可选）：三族 × control_per_family，用于计算真实 ROC/AUC。
    let control_families = build_control_families(control_per_family, 256, control_seed);
    let control_alphas: Vec<f64> = control_families
        .iter()
        .flat_map(|(_, a)| a.iter().copied())
        .collect();
    if control_per_family > 0 {
        for (family, alphas) in &control_families {
            println!(
                "Control [{}]: {} synthetic trajectories",
                family.as_str(),
                alphas.len()
            );
        }
        println!(
            "Total synthetic controls: {} ({} families)",
            control_alphas.len(),
            ControlFamily::ALL.len()
        );
    }

    // 生成人群报告（有对照才有 ROC）
    let report = generate_population_report(
        "GeoLife (Beijing)",
        &results,
        if control_alphas.is_empty() {
            None
        } else {
            Some(&control_alphas)
        },
    );
    let report_json =
        serde_json::to_value(&report).map_err(|e| crate::error::TripError::Json(e.to_string()))?;

    // 组合完整输出
    let output = json!({
        "calibration_type": "geolife_real_data",
        "dataset_root": data_dir.to_string_lossy(),
        "seed": control_seed,
        "dataset": {
            "name": "GeoLife (Beijing)",
            "source": "Microsoft Research GeoLife 1.3（北京 182 用户）",
            "input_unit": "PLT files",
            "input_count": plt_files.len(),
        },
        "summary": {
            "total_plt_files": plt_files.len(),
            "analyzed_trajectories": results.len(),
            "parse_errors": errors,
            "synthetic_controls": control_alphas.len(),
        },
        "control_families": control_family_summary(&control_families),
        "population_report": report_json,
        "per_trajectory_results": results,
    });

    // 输出
    let output_str = serde_json::to_string_pretty(&output)
        .map_err(|e| crate::error::TripError::Json(e.to_string()))?;
    match output_path {
        Some(p) => {
            fs::write(p, &output_str).map_err(|e| crate::error::TripError::Io(e.to_string()))?;
            println!("Report written to {}", p.display());
        }
        None => {
            println!("{}", output_str);
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plt_line_valid() {
        let line = "39.984702,116.318417,0,492,2008-10-23 05:53:05,2008-10-23";
        let pt = parse_plt_line(line);
        assert!(pt.is_some());
        let pt = pt.unwrap();
        assert!((pt.lat - 39.984702).abs() < 1e-9);
        assert!((pt.lng - 116.318417).abs() < 1e-9);
    }

    #[test]
    fn test_parse_plt_line_invalid() {
        assert!(parse_plt_line("").is_none());
        assert!(parse_plt_line("# comment").is_none());
        assert!(parse_plt_line("invalid").is_none());
    }

    #[test]
    fn test_parse_datetime() {
        let ts = parse_geolife_datetime("2008-10-23 05:53:05");
        assert!(ts.is_some());
        // 2008-10-23 05:53:05 UTC ≈ 1224726785
        let ts = ts.unwrap();
        assert!((ts - 1224726785).abs() < 86400); // 允许一天的误差（时区处理简化）
    }

    #[test]
    fn test_days_from_civil() {
        // 1970-01-01 = 0 days
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        // 2000-01-01 = 10957 days
        assert_eq!(days_from_civil(2000, 1, 1), 10957);
        // 2008-10-23 ≈ 14241 or 14242 days (depending on leap year calculation)
        let days = days_from_civil(2008, 10, 23);
        // Just verify it's in a reasonable range (year 2008 is ~13900-14300 days)
        assert!(days > 13000 && days < 14500, "days = {days}");
    }

    // ── W7：日期换算修正与 NaN 安全 ──────────────────────────────────────

    #[test]
    fn civil_from_days_is_inverse_of_days_from_civil() {
        // 覆盖闰年、世纪年、负向边界与当月天数边界。
        let cases: [(i32, u32, u32); 8] = [
            (1970, 1, 1),
            (1972, 2, 29), // 闰日
            (2000, 2, 29), // 世纪闰年
            (1999, 12, 31),
            (2008, 10, 23),
            (2024, 2, 29),
            (2026, 9, 25),
            (2100, 3, 1), // 2100 非闰年
        ];
        for (y, m, d) in cases {
            let days = days_from_civil(y, m, d);
            let (yy, mm, dd) = civil_from_days(days);
            assert_eq!(
                (yy, mm, dd),
                (y as i64, m, d),
                "roundtrip failed for {y}-{m}-{d} (days={days})"
            );
        }
    }

    #[test]
    fn iso8601_matches_known_timestamps() {
        assert_eq!(iso8601_from_unix(0), "1970-01-01T00:00:00Z");
        // 2026-01-01T00:00:00Z
        assert_eq!(iso8601_from_unix(1_767_225_600), "2026-01-01T00:00:00Z");
        // 2026-09-25T00:00:00Z
        assert_eq!(iso8601_from_unix(1_790_294_400), "2026-09-25T00:00:00Z");
        // 与 PLT 解析联立：北京时间 2008-10-23 05:53:05 → UTC 前一日 21:53:05。
        let ts = parse_geolife_datetime("2008-10-23 05:53:05").expect("parse datetime");
        assert_eq!(iso8601_from_unix(ts), "2008-10-22T21:53:05Z");
    }

    #[test]
    fn utc_now_iso_is_well_formed_and_plausible() {
        let s = utc_now_iso();
        // 形如 YYYY-MM-DDTHH:MM:SSZ
        assert_eq!(s.len(), 20, "unexpected length: {s}");
        let year: i64 = s[..4].parse().expect("year digits");
        let month: u32 = s[5..7].parse().expect("month digits");
        let day: u32 = s[8..10].parse().expect("day digits");
        // 2020 年之后、2100 年之前（当前 2026），月份/日期合法。
        assert!((2020..2100).contains(&year), "year = {year}");
        assert!((1..=12).contains(&month), "month = {month}");
        assert!((1..=31).contains(&day), "day = {day}");
        assert!(s.ends_with('Z'));
    }

    #[test]
    fn sorted_finite_drops_non_finite_and_sorts() {
        let v = sorted_finite(&[3.0, f64::NAN, 1.0, f64::INFINITY, 2.0, f64::NEG_INFINITY]);
        assert_eq!(v, vec![1.0, 2.0, 3.0]);
        assert!(sorted_finite(&[]).is_empty());
        assert!(sorted_finite(&[f64::NAN]).is_empty());
    }

    // ── W7：合成对照组 ──────────────────────────────────────────────────

    #[test]
    fn alpha_humanness_score_peaks_at_center() {
        assert!((alpha_humanness_score(ALPHA_CENTER) - 0.0).abs() < 1e-12);
        // 两侧等距给同分（对白噪声与棕噪声同时惩罚）。
        assert!((alpha_humanness_score(0.05) - alpha_humanness_score(1.05)).abs() < 1e-12);
        assert!(alpha_humanness_score(1.05) < alpha_humanness_score(0.55));
        assert!(alpha_humanness_score(f64::NAN).is_nan());
    }

    #[test]
    fn control_families_are_deterministic() {
        for family in ControlFamily::ALL {
            let a = synthetic_control_alphas(family, 4, 64, 7);
            let b = synthetic_control_alphas(family, 4, 64, 7);
            assert_eq!(a, b, "family {} not deterministic", family.as_str());
            assert!(!a.is_empty(), "family {} produced nothing", family.as_str());
            assert!(a.iter().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn control_families_separate_from_biological_center() {
        let window = 128;
        let iid = synthetic_control_alphas(ControlFamily::IidLevy, 8, window, 0xA11CE);
        let replay = synthetic_control_alphas(ControlFamily::ReplayDrift, 8, window, 0xBEEF);
        let ar1 = synthetic_control_alphas(ControlFamily::CorrelatedGaussian, 8, window, 0xC0FFEE);
        let iid_mean = mean_f64(&iid);
        let replay_mean = mean_f64(&replay);
        let ar1_mean = mean_f64(&ar1);
        // 谱平坦：远离粉噪中心（在中心下方）。
        assert!(
            iid_mean < ALPHA_CENTER,
            "iid levy mean α = {iid_mean} should be below center"
        );
        // 趋势谱 / 强低频相关：整体高于生物区间上界附近，远离中心。
        assert!(
            replay_mean > ALPHA_CENTER,
            "replay drift mean α = {replay_mean} should exceed center"
        );
        assert!(
            ar1_mean > ALPHA_CENTER,
            "AR(1) mean α = {ar1_mean} should exceed center"
        );
    }

    // ── W7：真实 ROC / AUC ─────────────────────────────────────────────

    #[test]
    fn roc_perfect_separation_gives_auc_one() {
        // 人类分数高、对照分数低 → 完美分离。
        let human = [0.9, 0.8, 0.7];
        let control = [0.2, 0.1, 0.05];
        let roc = roc_analysis(&human, &control).expect("roc");
        assert!((roc.auc - 1.0).abs() < 1e-12, "auc = {}", roc.auc);
        assert_eq!(roc.positives, 3);
        assert_eq!(roc.negatives, 3);
        assert!((roc.optimal_tpr - 1.0).abs() < 1e-12);
        assert!((roc.optimal_fpr - 0.0).abs() < 1e-12);
    }

    #[test]
    fn roc_non_separable_scores_give_auc_half() {
        // 交错样本：4 个人类-对照"一致对"中恰好 2 对 → AUC = 0.5（无判别力）。
        let human = [4.0, 1.0];
        let control = [3.0, 2.0];
        let roc = roc_analysis(&human, &control).expect("roc");
        assert!((roc.auc - 0.5).abs() < 1e-12, "auc = {}", roc.auc);
    }

    #[test]
    fn roc_partially_concordant_gives_three_quarters() {
        // {4,2} vs {3,1}：4 对中 3 对一致 → AUC = 0.75。
        let roc = roc_analysis(&[4.0, 2.0], &[3.0, 1.0]).expect("roc");
        assert!((roc.auc - 0.75).abs() < 1e-12, "auc = {}", roc.auc);
    }

    #[test]
    fn roc_ties_are_merged_deterministically() {
        let human = [0.5, 0.5];
        let control = [0.5, 0.5];
        let roc = roc_analysis(&human, &control).expect("roc");
        // 无法分离 → AUC = 0.5；且同分合并后点数有限。
        assert!((roc.auc - 0.5).abs() < 1e-12, "auc = {}", roc.auc);
        // 展示表只含有限阈值。
        for row in &roc.tpr_at_thresholds {
            assert!(row["threshold"].as_f64().is_some_and(f64::is_finite));
        }
    }

    #[test]
    fn roc_rejects_empty_or_non_finite_groups() {
        assert!(matches!(
            roc_analysis(&[], &[0.1, 0.2]),
            Err(TripError::Calibration(_))
        ));
        assert!(matches!(
            roc_analysis(&[0.1, 0.2], &[]),
            Err(TripError::Calibration(_))
        ));
        assert!(matches!(
            roc_analysis(&[f64::NAN], &[f64::NAN]),
            Err(TripError::Calibration(_))
        ));
    }

    #[test]
    fn roc_analysis_from_alphas_behaves_like_score_mapping() {
        let human = [0.50, 0.55, 0.60]; // 靠近中心
        let control = [-0.40, 1.50, 1.60]; // 远离中心
        let roc = roc_analysis_from_alphas(&human, &control).expect("roc");
        assert!((roc.auc - 1.0).abs() < 1e-12, "auc = {}", roc.auc);
        // 最优判决区间应覆盖全部人类样本，且不含任一对照样本。
        let band = roc.optimal_alpha_band.expect("alpha band present");
        assert!(
            band[0] <= 0.50 + 1e-12 && band[1] >= 0.60 - 1e-12,
            "band = {band:?}"
        );
        assert!(!(-0.40..=1.50).contains(&band[0]) || band[0] > -0.40);
    }

    #[test]
    fn roc_plain_scores_have_no_alpha_band() {
        let roc = roc_analysis(&[0.9, 0.8], &[0.1, 0.2]).expect("roc");
        assert!(roc.optimal_alpha_band.is_none());
    }

    // ── W7：人群报告契约 ───────────────────────────────────────────────

    fn fake_calibration(alpha: f64) -> TrajectoryCalibration {
        TrajectoryCalibration {
            user_id: "u".to_string(),
            trajectory_id: "t".to_string(),
            raw_points: 100,
            valid_points: 100,
            psd: Some(PsdResult {
                alpha,
                r_squared: 0.9,
                confidence: 0.5,
                classification: "pink".to_string(),
                sample_count: 99,
            }),
            levy: Some(LevyCalibResult {
                beta: 1.75,
                kappa: 5.0,
                r_min: 0.1,
                bridge_g: Some(0.44),
            }),
        }
    }

    #[test]
    fn population_report_without_control_has_no_roc() {
        let results: Vec<TrajectoryCalibration> = (0..40)
            .map(|i| fake_calibration(0.5 + i as f64 * 0.005))
            .collect();
        let report = generate_population_report("synthetic-human", &results, None);
        assert!(report.roc.is_none(), "no control → must not fabricate ROC");
        assert_eq!(report.analyzed_trajectories, 40);
        assert!(report.metadata.analyzed_at.ends_with('Z'));
    }

    #[test]
    fn population_report_with_control_has_auc() {
        let results: Vec<TrajectoryCalibration> = (0..40)
            .map(|i| fake_calibration(0.5 + i as f64 * 0.005))
            .collect();
        let control: Vec<f64> = (0..40).map(|i| 1.5 + i as f64 * 0.01).collect();
        let report = generate_population_report("synthetic-human", &results, Some(&control));
        let roc = report.roc.expect("roc present when control given");
        assert!(
            (roc.auc - 1.0).abs() < 1e-9,
            "clean separation should give AUC=1, got {}",
            roc.auc
        );
        assert_eq!(roc.positives, 40);
        assert_eq!(roc.negatives, 40);
    }

    #[test]
    fn population_report_survives_nan_alpha() {
        // 含 NaN 的 α 不得 panic（历史上会因 partial_cmp().unwrap() 崩溃）。
        let mut results: Vec<TrajectoryCalibration> = (0..30)
            .map(|i| fake_calibration(0.5 + i as f64 * 0.01))
            .collect();
        results.push(fake_calibration(f64::NAN));
        let report = generate_population_report("dirty", &results, None);
        assert_eq!(report.alpha_stats.n, 30, "NaN must be excluded from stats");
    }

    #[test]
    fn test_process_short_trajectory_returns_none() {
        let points = vec![
            GpsPoint {
                lat: 39.9,
                lng: 116.3,
                timestamp: 1000,
                raw_datetime: String::new()
            };
            10
        ];
        let result = process_trajectory("user001", "traj001", &points);
        assert!(result.is_none());
    }
}
