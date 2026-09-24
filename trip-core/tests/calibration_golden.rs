//! W7 标定端到端黄金测试（离线、确定性、可回归）。
//!
//! 本测试把「合成人类轨迹 + 三族合成攻击对照组」跑完整标定管线，并把
//! 关键聚合量固化到 `tests/vectors/calibration-vectors.json`：
//!
//! - 人类组 α 分布（n / mean / std / 各分位）
//! - 生物区间覆盖率
//! - 逐族对照组的 α 统计
//! - 真实 ROC 的 AUC / 最优点 / α 判决区间
//!
//! 与 `engine_golden.rs` 的区别：那里锁的是**逐字节**数值管线（跨语言可复现），
//! 这里锁的是**统计聚合量**——生成器依赖 `StdRng`(ChaCha12) 与 H3 量化，
//! 不做跨语言位级承诺，但必须同平台逐次可复现、且不随重构漂移。
//!
//! 重新生成向量（仅在有意变更生成器/引擎时使用）：
//!
//! ```bash
//! UPDATE_CALIBRATION_VECTORS=1 cargo test -p trip-core --test calibration_golden
//! ```

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::{json, Value};

use trip_core::engine::calibration::{
    build_control_families, calibrate_trajectory, control_family_summary,
    generate_population_report, ProcessedTrajectory, TrajectoryCalibration,
};
use trip_core::engine::psd::displacements_from_cells;
use trip_core::engine::sim::{normal, trip_walk_path, SimConfig, TripConfig};

// ── 场景参数（固定值 = 可复现） ────────────────────────────────────────
const SEED: u64 = 24301; // == 0x5EED
const WINDOW: usize = 256;
const HUMANS: usize = 60;
const CONTROL_PER_FAMILY: usize = 60;
const VECTORS_PATH: &str = "/tests/vectors/calibration-vectors.json";

/// 浮点比较容差（同平台应逐位一致；留余量以防编译器差异）。
const TOL: f64 = 1e-9;

/// 生成合成"人类"轨迹并标定（与 `gyid calibrate synth` 完全一致的流程）。
fn synthesize_humans() -> Vec<TrajectoryCalibration> {
    let cfg = SimConfig {
        raw_step_cap: 40_000,
        ..SimConfig::default()
    };
    let trip = TripConfig::default();
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut results = Vec::with_capacity(HUMANS);
    for i in 0..HUMANS {
        let beta = 1.50 + 0.40 * rng.gen::<f64>();
        let kappa = 5.0 * normal(&mut rng).exp();
        let path = trip_walk_path(&mut rng, beta, kappa, WINDOW, &cfg, &trip);
        let displacements = displacements_from_cells(&path.cells).expect("displacements");
        let traj = ProcessedTrajectory {
            user_id: "golden".to_string(),
            trajectory_id: format!("human-{i:04}"),
            cells: path.cells.clone(),
            displacements,
            raw_point_count: path.raw_steps as usize,
            valid_point_count: path.cells.len(),
        };
        results.push(calibrate_trajectory(&traj));
    }
    results
}

/// 计算完整场景，返回 (报告 JSON, 对照组逐族 JSON)。
fn run_scenario() -> (Value, Value) {
    let results = synthesize_humans();
    let families = build_control_families(CONTROL_PER_FAMILY, WINDOW, SEED);
    let control_alphas: Vec<f64> = families
        .iter()
        .flat_map(|(_, a)| a.iter().copied())
        .collect();
    let report = generate_population_report(
        "Synthetic (trip_walk)",
        &results,
        if control_alphas.is_empty() {
            None
        } else {
            Some(&control_alphas)
        },
    );
    let report_json = serde_json::to_value(&report).expect("serialize report");
    let families_json = control_family_summary(&families);
    (report_json, families_json)
}

/// 从报告与逐族统计中抽取"黄金向量"字段。
fn extract_vectors(report: &Value, families: &Value) -> Value {
    let alpha = &report["alpha_stats"];
    let roc = &report["roc"];
    json!({
        "scenario": {
            "seed": SEED,
            "window": WINDOW,
            "humans": HUMANS,
            "control_per_family": CONTROL_PER_FAMILY,
        },
        "human_alpha": {
            "n": alpha["n"],
            "mean": alpha["mean"],
            "std": alpha["std"],
            "median": alpha["median"],
            "p05": alpha["p05"],
            "p25": alpha["p25"],
            "p75": alpha["p75"],
            "p95": alpha["p95"],
            "min": alpha["min"],
            "max": alpha["max"],
            "in_draft_bio_range": alpha["in_draft_bio_range"],
        },
        "bio_fraction": report["bio_fraction"],
        "control_families": families,
        "roc": {
            "auc": roc["auc"],
            "positives": roc["positives"],
            "negatives": roc["negatives"],
            "optimal_threshold": roc["optimal_threshold"],
            "optimal_tpr": roc["optimal_tpr"],
            "optimal_fpr": roc["optimal_fpr"],
            "optimal_alpha_band": roc["optimal_alpha_band"],
        },
    })
}

fn load_vectors() -> Value {
    let path = format!("{}{}", env!("CARGO_MANIFEST_DIR"), VECTORS_PATH);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {path}: {e}\n(用 UPDATE_CALIBRATION_VECTORS=1 生成)"));
    serde_json::from_str(&text).expect("valid JSON")
}

fn store_vectors(v: &Value) {
    let path = format!("{}{}", env!("CARGO_MANIFEST_DIR"), VECTORS_PATH);
    let text = serde_json::to_string_pretty(v).expect("serialize") + "\n";
    std::fs::write(&path, text).unwrap_or_else(|e| panic!("write {path}: {e}"));
    eprintln!("✓ calibration vectors written to {path}");
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= TOL,
        "{label}: actual {actual} vs expected {expected} (tol {TOL})"
    );
}

// ── 测试 ───────────────────────────────────────────────────────────────

#[test]
fn calibration_aggregates_match_recorded_vectors() {
    let (report, families) = run_scenario();
    let vectors = extract_vectors(&report, &families);

    if std::env::var("UPDATE_CALIBRATION_VECTORS").is_ok() {
        store_vectors(&vectors);
        return;
    }

    let recorded = load_vectors();

    // 场景参数必须一致（防止无意改动导致向量失效）
    assert_eq!(
        vectors["scenario"], recorded["scenario"],
        "scenario parameters drifted; regenerate vectors if intentional"
    );

    // 人类组 α 统计
    for key in [
        "mean",
        "std",
        "median",
        "p05",
        "p25",
        "p75",
        "p95",
        "min",
        "max",
        "in_draft_bio_range",
    ] {
        assert_close(
            vectors["human_alpha"][key].as_f64().expect("f64"),
            recorded["human_alpha"][key].as_f64().expect("f64"),
            &format!("human_alpha.{key}"),
        );
    }
    assert_eq!(
        vectors["human_alpha"]["n"].as_u64(),
        recorded["human_alpha"]["n"].as_u64()
    );
    assert_close(
        vectors["bio_fraction"].as_f64().expect("f64"),
        recorded["bio_fraction"].as_f64().expect("f64"),
        "bio_fraction",
    );

    // 逐族对照
    for family in ["iid_levy", "replay_drift", "correlated_gaussian"] {
        for key in ["mean", "median", "p05", "p95"] {
            assert_close(
                vectors["control_families"][family][key]
                    .as_f64()
                    .expect("f64"),
                recorded["control_families"][family][key]
                    .as_f64()
                    .expect("f64"),
                &format!("control_families.{family}.{key}"),
            );
        }
    }

    // ROC
    assert_close(
        vectors["roc"]["auc"].as_f64().expect("f64"),
        recorded["roc"]["auc"].as_f64().expect("f64"),
        "roc.auc",
    );
    assert_close(
        vectors["roc"]["optimal_threshold"].as_f64().expect("f64"),
        recorded["roc"]["optimal_threshold"].as_f64().expect("f64"),
        "roc.optimal_threshold",
    );
    assert_eq!(
        vectors["roc"]["positives"].as_u64(),
        recorded["roc"]["positives"].as_u64()
    );
    assert_eq!(
        vectors["roc"]["negatives"].as_u64(),
        recorded["roc"]["negatives"].as_u64()
    );
}

/// 去掉墙钟时间戳（`metadata.analyzed_at`），只留确定性部分。
fn strip_volatile(report: &Value) -> Value {
    let mut v = report.clone();
    if let Some(meta) = v.get_mut("metadata").and_then(Value::as_object_mut) {
        meta.remove("analyzed_at");
    }
    v
}

#[test]
fn calibration_is_deterministic_across_runs() {
    let (a, fa) = run_scenario();
    let (b, fb) = run_scenario();
    assert_eq!(
        serde_json::to_string(&strip_volatile(&a)).expect("ser"),
        serde_json::to_string(&strip_volatile(&b)).expect("ser"),
        "identical seed must yield identical reports (excluding wall-clock timestamp)"
    );
    assert_eq!(
        serde_json::to_string(&fa).expect("ser"),
        serde_json::to_string(&fb).expect("ser"),
        "control family summary must be deterministic"
    );
}

#[test]
fn calibration_report_json_roundtrips() {
    let (report, _) = run_scenario();
    let text = serde_json::to_string(&report).expect("serialize");
    let back: Value = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(
        report["analyzed_trajectories"],
        back["analyzed_trajectories"]
    );
    assert_eq!(report["alpha_stats"]["n"], back["alpha_stats"]["n"]);
    // serde_json 默认的浮点解析不保证逐位往返（除非启用 float_roundtrip
    // feature）；对报告传输而言 1 ULP 无实质影响，按容差比较。
    assert_close(
        report["roc"]["auc"].as_f64().expect("f64"),
        back["roc"]["auc"].as_f64().expect("f64"),
        "roc.auc roundtrip",
    );
    assert_close(
        report["alpha_stats"]["mean"].as_f64().expect("f64"),
        back["alpha_stats"]["mean"].as_f64().expect("f64"),
        "alpha_stats.mean roundtrip",
    );
}

#[test]
fn generator_pair_separates_with_high_auc() {
    // 结构性断言（不依赖向量文件）：结构化 Levy 人类 vs 三族攻击，
    // 单统计量 α 应给出强判别力。
    let (report, _) = run_scenario();
    let auc = report["roc"]["auc"].as_f64().expect("auc");
    assert!(auc > 0.85, "expected strong separation, got AUC = {auc}");

    let band = report["roc"]["optimal_alpha_band"]
        .as_array()
        .expect("alpha band");
    assert_eq!(band.len(), 2);
    let (lo, hi) = (
        band[0].as_f64().expect("f64"),
        band[1].as_f64().expect("f64"),
    );
    assert!(lo < hi, "band must be ordered: [{lo}, {hi}]");
    // 判决区间应覆盖生物区间中心。
    assert!(
        (lo..=hi).contains(&0.55),
        "band [{lo}, {hi}] must contain 0.55"
    );
}

#[test]
fn no_control_group_means_no_roc() {
    let results = synthesize_humans();
    let report = generate_population_report("Synthetic (trip_walk)", &results, None);
    assert!(
        report.roc.is_none(),
        "must not fabricate ROC without a control group"
    );
    assert_eq!(report.analyzed_trajectories, results.len());
    assert!(report.metadata.analyzed_at.ends_with('Z'));
    assert!(report.alpha_stats.n > 0);
}
