//! GeoLife/MDC 标定示例（W7）。
//!
//! 用法:
//!
//! ```bash
//! # 对 GeoLife 数据集运行完整标定（默认不算 ROC）
//! cargo run --release -p trip-core --example geolife_calibration /path/to/GeoLife_Trajectories_1.3 [output.json]
//!
//! # 生成合成对照组并计算真实 ROC / AUC（每族 200 条）
//! cargo run --release -p trip-core --example geolife_calibration ./data/GeoLife_Trajectories_1.3 report.json --control 200
//!
//! # 仅显示帮助
//! cargo run --release -p trip-core --example geolife_calibration -- --help
//! ```
//!
//! `--control N` 会为三族合成攻击轨迹（iid_levy / replay_drift /
//! correlated_gaussian）各生成 N 条，用于计算"人类 vs 机器"的判别 AUC。
//! `--seed S` 固定对照组随机种子（默认 0x5EED），保证结果可复现。

use std::env;
use std::path::{Path, PathBuf};

const DEFAULT_CONTROL_SEED: u64 = 0x5EED;

const HELP: &str = "\
GeoLife/MDC Calibration (W7)

Usage:
  geolife_calibration <data_dir> [output.json] [--control N] [--seed S]

Arguments:
  data_dir        Path to GeoLife_Trajectories_1.3 directory
  output.json     Optional output path (default: stdout)

Options:
  --control N     Synthetic control trajectories per family (default: 0 = no ROC)
                  Families: iid_levy, replay_drift, correlated_gaussian
  --seed S        Control RNG seed (default: 14693 / 0x5EED)

Examples:
  geolife_calibration ./data/GeoLife_Trajectories_1.3 report.json
  geolife_calibration ./data/GeoLife_Trajectories_1.3 report.json --control 200
  geolife_calibration ./data/GeoLife_Trajectories_1.3 report.json --control 200 --seed 42
";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        std::process::exit(0);
    }

    // 位置参数：data_dir [output.json]；其余为 --control/--seed。
    let mut positional: Vec<&String> = Vec::new();
    let mut control_per_family: usize = 0;
    let mut seed: u64 = DEFAULT_CONTROL_SEED;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--control" => {
                i += 1;
                control_per_family = match args.get(i).and_then(|s| s.parse().ok()) {
                    Some(n) => n,
                    None => {
                        eprintln!("Error: --control requires a non-negative integer");
                        std::process::exit(2);
                    }
                };
            }
            "--seed" => {
                i += 1;
                seed = match args.get(i).and_then(|s| s.parse().ok()) {
                    Some(n) => n,
                    None => {
                        eprintln!("Error: --seed requires an unsigned integer");
                        std::process::exit(2);
                    }
                };
            }
            other if other.starts_with("--") => {
                eprintln!("Error: unknown option {other}\n\n{HELP}");
                std::process::exit(2);
            }
            _ => positional.push(&args[i]),
        }
        i += 1;
    }

    let data_dir = match positional.first() {
        Some(p) => Path::new(p.as_str()),
        None => {
            eprintln!("Error: missing <data_dir>\n\n{HELP}");
            std::process::exit(2);
        }
    };
    let output_path: Option<PathBuf> = positional.get(1).map(|p| PathBuf::from(p.as_str()));

    if !data_dir.exists() {
        eprintln!(
            "Error: Data directory does not exist: {}",
            data_dir.display()
        );
        std::process::exit(1);
    }

    if control_per_family > 0 {
        println!("Synthetic control: {control_per_family} per family × 3 families (seed={seed})");
    }

    match trip_core::engine::calibration::run_geolife_calibration(
        data_dir,
        output_path.as_deref(),
        control_per_family,
        seed,
    ) {
        Ok(_) => {
            println!("\nCalibration completed successfully.");
        }
        Err(e) => {
            eprintln!("Calibration failed: {e}");
            std::process::exit(1);
        }
    }
}
