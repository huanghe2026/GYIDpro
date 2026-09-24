//! `gyid calibrate …`：W7 人群标定与判别能力报告（GYIP-0003 §8 W7）。
//!
//! 三个子命令，覆盖从"零依赖跑通"到"接真实数据集"的完整链路：
//!
//! - `synth`：**全离线**。用 `engine::sim` 生成合成"人类"轨迹（trip_walk）
//!   与三族合成攻击轨迹，跑完整标定 + 真实 ROC/AUC。无需任何外部数据，
//!   适合 CI 与快速自检；
//! - `geolife`：接真实 GeoLife 数据目录（PLT），可选叠加合成对照组求 AUC；
//! - `whitepaper`：把标定 JSON 报告渲染成 Markdown 白皮书草稿。
//!
//! 统计全部复用 `trip-core::engine::calibration`（与线上判定同一份引擎），
//! 本模块只负责文件 I/O、参数解析与进度输出——与 `anchor_cmds.rs` 的组织
//! 方式保持一致。

use clap::Subcommand;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use trip_core::engine::calibration::{
    build_control_families, calibrate_trajectory, control_family_summary,
    generate_population_report, ControlFamily, PopulationCalibrationReport, ProcessedTrajectory,
    TrajectoryCalibration,
};
use trip_core::engine::psd::displacements_from_cells;
use trip_core::engine::sim::{normal, trip_walk_path, SimConfig, TripConfig};

/// 对照组默认随机种子（固定值 → 可复现）。
const DEFAULT_SEED: u64 = 0x5EED;
/// 默认 PSD 窗口（位移样本数，草案推荐 256）。
const DEFAULT_WINDOW: usize = 256;
/// 对照组生成的窗口（与标定窗口一致的推荐值）。
const CONTROL_WINDOW: usize = 256;

#[derive(Subcommand)]
pub enum CalibrateCmd {
    /// 全合成离线标定（无需外部数据；适合 CI 与快速自检）
    Synth {
        /// 合成"人类"轨迹数（trip_walk 结构化 Levy 生成器）
        #[arg(long, default_value = "60")]
        humans: usize,
        /// 每族合成攻击轨迹数（共 3 族）
        #[arg(long, default_value = "60")]
        control: usize,
        /// PSD 窗口（位移样本数，≥64）
        #[arg(long, default_value_t = DEFAULT_WINDOW)]
        window: usize,
        /// 随机种子（固定 → 可复现）
        #[arg(long, default_value_t = DEFAULT_SEED)]
        seed: u64,
        /// JSON 报告输出路径（缺省打印到 stdout）
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// 对真实 GeoLife 数据目录运行标定
    Geolife {
        /// GeoLife_Trajectories_* 根目录
        #[arg(long)]
        data_dir: PathBuf,
        /// 每族合成对照轨迹数（0 = 不计算 ROC）
        #[arg(long, default_value = "0")]
        control: usize,
        /// 对照组随机种子
        #[arg(long, default_value_t = DEFAULT_SEED)]
        seed: u64,
        /// JSON 报告输出路径（缺省打印到 stdout）
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// 把标定 JSON 报告渲染为 Markdown 白皮书草稿
    Whitepaper {
        /// 标定 JSON（由 `synth` / `geolife` 产出）
        #[arg(long)]
        input: PathBuf,
        /// Markdown 输出路径
        #[arg(long, default_value = "docs/CALIBRATION-W7-WHITEPAPER.md")]
        output: PathBuf,
    },
}

/// 入口：由 `main.rs` 的 `Cmd::Calibrate` 转发。
pub fn dispatch(cmd: CalibrateCmd) {
    match cmd {
        CalibrateCmd::Synth {
            humans,
            control,
            window,
            seed,
            output,
        } => cmd_synth(humans, control, window, seed, output),
        CalibrateCmd::Geolife {
            data_dir,
            control,
            seed,
            output,
        } => cmd_geolife(&data_dir, control, seed, output),
        CalibrateCmd::Whitepaper { input, output } => cmd_whitepaper(&input, &output),
    }
}

/// `calibrate synth`：全合成离线跑通。
fn cmd_synth(humans: usize, control: usize, window: usize, seed: u64, output: Option<PathBuf>) {
    if humans == 0 {
        fail("--humans 必须 ≥ 1");
    }
    if window < 64 {
        fail("--window 必须 ≥ 64（PSD 最小样本数）");
    }

    println!("=== 合成标定（离线） ===");
    println!(
        "人类轨迹 = {humans}，对照 = {control}/族 × {} 族，窗口 = {window}，seed = {seed}",
        ControlFamily::ALL.len()
    );

    // 1) 合成"人类"轨迹：结构化 Levy（trip_walk），α 落在生物区间附近。
    let cfg = SimConfig {
        raw_step_cap: 40_000,
        ..SimConfig::default()
    };
    let trip = TripConfig::default();
    let mut rng = StdRng::seed_from_u64(seed);
    let mut results: Vec<TrajectoryCalibration> = Vec::with_capacity(humans);
    for i in 0..humans {
        // β 在草案人类区间内取样；κ 对数正态。
        let beta = 1.50 + 0.40 * rng.gen::<f64>();
        let kappa = 5.0 * normal(&mut rng).exp();
        let path = trip_walk_path(&mut rng, beta, kappa, window, &cfg, &trip);
        let displacements = match displacements_from_cells(&path.cells) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let traj = ProcessedTrajectory {
            user_id: "synth".to_string(),
            trajectory_id: format!("human-{i:04}"),
            cells: path.cells.clone(),
            displacements,
            raw_point_count: path.raw_steps as usize,
            valid_point_count: path.cells.len(),
        };
        results.push(calibrate_trajectory(&traj));
    }
    println!("已标定人类轨迹：{}", results.len());

    // 2) 合成对照组（三族，互相独立的派生 seed）。
    let families = build_control_families(control, CONTROL_WINDOW, seed);
    let control_alphas: Vec<f64> = families
        .iter()
        .flat_map(|(_, a)| a.iter().copied())
        .collect();
    if control > 0 {
        for (family, alphas) in &families {
            println!("对照 [{}]：{} 条", family.as_str(), alphas.len());
        }
    }

    // 3) 人群报告（有对照才计算 ROC）。
    let report = generate_population_report(
        "Synthetic (trip_walk)",
        &results,
        if control_alphas.is_empty() {
            None
        } else {
            Some(&control_alphas)
        },
    );

    let dataset = json!({
        "name": "Synthetic (trip_walk)",
        "source": "trip-core engine::sim（结构化 Levy 人类 + 三族合成攻击）",
        "input_unit": "synthetic trajectories",
        "input_count": results.len(),
    });
    let out = assemble_output(
        "synthetic",
        dataset,
        &results,
        &report,
        control_family_summary(&families),
        control_alphas.len(),
        seed,
    );

    emit(&out, output.as_deref());
}

/// `calibrate geolife`：真实数据集（复用 trip-core 的批量入口）。
fn cmd_geolife(data_dir: &Path, control: usize, seed: u64, output: Option<PathBuf>) {
    if !data_dir.exists() {
        fail(&format!("数据目录不存在：{}", data_dir.display()));
    }
    if !data_dir.is_dir() {
        fail(&format!("不是目录：{}", data_dir.display()));
    }
    match trip_core::engine::calibration::run_geolife_calibration(
        data_dir,
        output.as_deref(),
        control,
        seed,
    ) {
        Ok(_) => {
            if output.is_some() {
                println!("✓ 标定完成");
            }
            // output 为 None 时报告已由 trip-core 打印到 stdout。
        }
        Err(e) => fail(&format!("标定失败：{e}")),
    }
}

/// `calibrate whitepaper`：JSON → Markdown。
fn cmd_whitepaper(input: &Path, output: &Path) {
    let text = match std::fs::read_to_string(input) {
        Ok(t) => t,
        Err(e) => fail(&format!("无法读取 {}：{e}", input.display())),
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => fail(&format!("{} 不是合法 JSON：{e}", input.display())),
    };
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                fail(&format!("无法创建目录 {}：{e}", parent.display()));
            }
        }
    }
    match trip_core::engine::calibration_report::generate_whitepaper(&value, output) {
        Ok(()) => println!("✓ 白皮书已写入 {}", output.display()),
        Err(e) => fail(&format!("生成白皮书失败：{e}")),
    }
}

/// 组装与 `run_geolife_calibration` 同构的输出 JSON（供白皮书消费）。
fn assemble_output(
    calibration_type: &str,
    dataset: Value,
    results: &[TrajectoryCalibration],
    report: &PopulationCalibrationReport,
    control_families: Value,
    control_count: usize,
    seed: u64,
) -> Value {
    let report_json = match serde_json::to_value(report) {
        Ok(v) => v,
        Err(e) => fail(&format!("报告序列化失败：{e}")),
    };
    json!({
        "calibration_type": calibration_type,
        "seed": seed,
        "dataset": dataset,
        "summary": {
            "analyzed_trajectories": results.len(),
            "parse_errors": 0,
            "synthetic_controls": control_count,
        },
        "control_families": control_families,
        "population_report": report_json,
        "per_trajectory_results": results,
    })
}

/// 输出到文件或 stdout。
fn emit(value: &Value, output: Option<&Path>) {
    let text = match serde_json::to_string_pretty(value) {
        Ok(t) => t,
        Err(e) => fail(&format!("JSON 序列化失败：{e}")),
    };
    match output {
        Some(p) => {
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        fail(&format!("无法创建目录 {}：{e}", parent.display()));
                    }
                }
            }
            if let Err(e) = std::fs::write(p, &text) {
                fail(&format!("写入 {} 失败：{e}", p.display()));
            }
            println!("✓ 报告已写入 {}", p.display());
        }
        None => println!("{text}"),
    }
}

/// 统一错误出口。
fn fail(msg: &str) -> ! {
    eprintln!("✗ {msg}");
    std::process::exit(1);
}
