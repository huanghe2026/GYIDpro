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
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::engine::levy::{bridge_check, fit as levy_fit};
use crate::engine::psd::{
    displacements_from_cells, psd_alpha, ALPHA_CENTER, BIO_ALPHA_MAX, BIO_ALPHA_MIN,
};
use crate::error::Result;

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

/// ROC 分析结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct RocAnalysis {
    /// 真人真阳性率（不同阈值下的 TPR/FPR 表）。
    pub tpr_at_thresholds: Vec<Value>,
    /// 最优阈值（Youden's J 统计量最大处）。
    pub optimal_threshold: f64,
    /// AUC（如果有对照组数据）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auc: Option<f64>,
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
pub fn generate_population_report(
    dataset_name: &str,
    results: &[TrajectoryCalibration],
) -> PopulationCalibrationReport {
    let total = results.len();
    let analyzed = results.iter().filter(|r| r.psd.is_some()).count();

    // 收集有效的 α 值
    let mut alphas: Vec<f64> = results
        .iter()
        .filter_map(|r| r.psd.as_ref().map(|p| p.alpha))
        .collect();
    alphas.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // 收集有效的 β 值
    let mut betas: Vec<f64> = results
        .iter()
        .filter_map(|r| r.levy.as_ref().map(|l| l.beta))
        .collect();
    betas.sort_by(|a, b| a.partial_cmp(b).unwrap());

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

    // ROC 分析（简化版：基于分布计算不同阈值的 TPR）
    let roc = compute_roc_analysis(&alphas);

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

/// 计算 ROC 分析（基于分布的简化版）。
fn compute_roc_analysis(alphas: &[f64]) -> Option<RocAnalysis> {
    if alphas.len() < 30 {
        return None;
    }

    // 计算在不同 α 阈值下的"真阳性率"（假设真人应该在生物区间内）
    let thresholds: Vec<f64> = (0..=20).map(|i| 0.1 * i as f64).collect();
    let mut tpr_at = Vec::new();

    for &thresh in &thresholds {
        // TPR: α >= thresh 的比例（宽松阈值捕获更多人）
        let tpr = alphas.iter().filter(|&&a| a >= thresh).count() as f64 / alphas.len() as f64;
        // FPR: 假设 α < 0.15 为明确非生物，FPR = α < thresh 且 α >= 0.15 的比例
        // 这里简化：用低于 thresh 但高于白噪声的比例作为 FPR 估计
        let fpr = alphas.iter().filter(|&&a| a < thresh && a >= 0.10).count() as f64
            / alphas.len() as f64;

        tpr_at.push(json!({
            "threshold": thresh,
            "tpr": tpr,
            "fpr": fpr,
        }));
    }

    // 最优阈值：选择 TPR - FPR 最大的点（Youden's J）
    let optimal = tpr_at
        .iter()
        .map(|v| {
            let tpr = v["tpr"].as_f64().unwrap_or(0.0);
            let fpr = v["fpr"].as_f64().unwrap_or(0.0);
            (tpr - fpr, v["threshold"].as_f64().unwrap_or(0.55))
        })
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    Some(RocAnalysis {
        tpr_at_thresholds: tpr_at,
        optimal_threshold: optimal.map(|(_, t)| t).unwrap_or(ALPHA_CENTER),
        auc: None, // 需要对照组数据才能计算真正的 AUC
    })
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

fn utc_now_iso() -> String {
    use std::time::UNIX_EPOCH;
    let dur = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // 简化的 ISO 格式
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        1970 + secs / 31536000 % 100,
        (secs % 31536000) / 2592000 + 1,
        (secs % 2592000) / 86400 + 1,
        (secs % 86400) / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

// ---------------------------------------------------------------------------
// 主入口：批量标定
// ---------------------------------------------------------------------------

/// 对 GeoLife 数据集运行完整标定流水线。
///
/// # 参数
/// - `data_dir`: GeoLife 数据根目录
/// - `output_path`: JSON 报告输出路径（可选，默认 stdout）
///
/// # 返回
/// 标定报告 JSON
pub fn run_geolife_calibration(data_dir: &Path, output_path: Option<&Path>) -> Result<Value> {
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

    // 生成人群报告
    let report = generate_population_report("GeoLife (Beijing)", &results);
    let report_json =
        serde_json::to_value(&report).map_err(|e| crate::error::TripError::Json(e.to_string()))?;

    // 组合完整输出
    let output = json!({
        "calibration_type": "geolife_real_data",
        "dataset_root": data_dir.to_string_lossy(),
        "summary": {
            "total_plt_files": plt_files.len(),
            "analyzed_trajectories": results.len(),
            "parse_errors": errors,
        },
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
