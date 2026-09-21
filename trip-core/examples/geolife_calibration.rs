//! GeoLife/MDC 标定示例（W7）。
//!
//! 用法:
//!
//! ```bash
//! # 对 GeoLife 数据集运行完整标定
//! cargo run --release -p trip-core --example geolife_calibration /path/to/GeoLife_Trajectories_1.3 [output.json]
//!
//! # 仅显示帮助
//! cargo run --release -p trip-core --example geolife_calibration -- --help
//! ```

use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.get(1).map(|s| s.as_str()) == Some("--help") {
        println!("GeoLife/MDC Calibration (W7)");
        println!();
        println!("Usage:");
        println!("  geolife_calibration <data_dir> [output.json]");
        println!();
        println!("Arguments:");
        println!("  data_dir    Path to GeoLife_Trajectories_1.3 directory");
        println!("  output.json Optional output path (default: stdout)");
        println!();
        println!("Example:");
        println!("  geolife_calibration ./data/GeoLife_Trajectories_1.3 report.json");
        std::process::exit(0);
    }

    let data_dir = Path::new(&args[1]);
    let output_path = args.get(2).map(Path::new);

    if !data_dir.exists() {
        eprintln!(
            "Error: Data directory does not exist: {}",
            data_dir.display()
        );
        std::process::exit(1);
    }

    match trip_core::engine::calibration::run_geolife_calibration(data_dir, output_path) {
        Ok(_) => {
            println!("\nCalibration completed successfully.");
        }
        Err(e) => {
            eprintln!("Calibration failed: {e}");
            std::process::exit(1);
        }
    }
}
