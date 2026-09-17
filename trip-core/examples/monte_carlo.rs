//! Monte Carlo 数值验证（draft-04 §7.3.3 Numerical Validation）+ GeoYuan 扩展。
//!
//! 五组轨迹，全部确定性种子（同机重跑逐位一致）：
//!
//! 1. `levy_iid`：草案 §7.3.3 字面生成器——i.i.d. 截断 Levy 步进
//!    （β~U[1.50,1.90]，κ~对数正态），随机航向 + H3 res10 量化 + §4.1 去重。
//!    **实证结论**：i.i.d. 空间步进序列的位移谱是平坦的（α≈0），桥校验
//!    g=α/(3−β) 系统性落入 [0.3,0.7] 之外——1/f 临界性不可能来自无记忆的
//!    空间模型，只能来自活动的时间爆发结构（Barabási 人类动力学）。
//!    本组如实报告，作为给上游 rats@ietf.org 的 draft feedback 证据。
//! 2. `trip_walk`：GeoYuan 扩展——出行结构化 Levy 轨迹（Levy 出行距离
//!    拆分为 n 个相关子步，见 [`trip_core::engine::sim::TripConfig`]）。
//!    目标：中位 α≈0.5–0.6（桥中心 g≈0.44）。默认参数由 `--calibrate`
//!    校准后冻结。实证否证：i.i.d. 步进谱平坦（α≈0）；线性/对数域
//!    幅度包络调制也被重尾噪声与 §4.1 去重门控消灭（α≈0）。
//! 3. `random_walk`：白噪声位移（预期 α≈0）。
//! 4. `replay_with_drift`：确定性重放 + 线性漂移（预期 α≈2）。
//! 5. `correlated_gaussian`：AR(1) 速度积分（预期 α 在生物区间外的高频端）。
//!
//! 运行（release）：
//!
//! ```bash
//! # 全量验证（默认 10,000 条/组，输出 JSON 报告到指定路径或 stdout）
//! cargo run --release -p trip-core --example monte_carlo [输出.json]
//! # contrast 扫描（每组 300 条），为 bursty 默认参数选点
//! cargo run --release -p trip-core --example monte_carlo -- --calibrate [输出.json]
//! ```

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::{json, Value};
use trip_core::engine::levy::{bridge_check, fit, LevyParams};
use trip_core::engine::psd::{displacements_from_cells, psd_alpha};
use trip_core::engine::sim::{levy_path, normal, trip_walk_path, SimConfig, TripConfig};

/// 每组轨迹数（草案指定 Levy 组为 10,000；其余组同规模便于对比）。
const TRAJECTORIES: usize = 10_000;
/// `--calibrate` 模式下每个 contrast 的轨迹数。
const CALIBRATION_TRAJECTORIES: usize = 300;
/// PSD 分析窗口（位移样本数，推荐值 256）。
const WINDOW: usize = 256;
/// κ 对数正态参数（GeoYuan 城市人群默认，W7 将用 GeoLife/T-Drive 标定）。
const KAPPA_MEDIAN_KM: f64 = 5.0;
const KAPPA_SIGMA: f64 = 1.0;
/// 重放组的线性漂移（km/步）：模拟重放设备的累积位置蠕变。
const REPLAY_DRIFT_KM: f64 = 0.05;
/// 重放组的 GPS 抖动（km）：让 10k 条重放轨迹的 α 有自然分布。
const REPLAY_JITTER_KM: f64 = 1e-4;

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

fn alpha_stats(alphas: &mut [f64]) -> Value {
    alphas.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = alphas.len() as f64;
    let mean = alphas.iter().sum::<f64>() / n;
    let var = alphas.iter().map(|a| (a - mean).powi(2)).sum::<f64>() / n;
    let frac_bio = alphas.iter().filter(|a| (0.30..=0.80).contains(*a)).count() as f64 / n;
    json!({
        "mean": mean,
        "std": var.sqrt(),
        "p05": percentile(alphas, 0.05),
        "p50": percentile(alphas, 0.50),
        "p95": percentile(alphas, 0.95),
        "frac_in_bio_030_080": frac_bio,
    })
}

/// 桥校验 g 序列统计（g = α/(3−β_true)，草案 §7.3 要求 ∈ [0.3, 0.7]）。
fn g_stats(gs: &mut [f64]) -> Value {
    gs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = gs.len() as f64;
    let frac = gs.iter().filter(|g| (0.3..=0.7).contains(*g)).count() as f64 / n;
    json!({
        "p05": percentile(gs, 0.05),
        "p50": percentile(gs, 0.50),
        "p95": percentile(gs, 0.95),
        "frac_in_03_07": frac,
    })
}

/// 人类移动参数抽签：β ~ U[1.50,1.90]，κ ~ 对数正态（中位数 5 km，σ=1）。
fn draw_mobility(rng: &mut impl Rng) -> (f64, f64) {
    let beta = 1.50 + 0.40 * rng.gen::<f64>();
    let kappa = KAPPA_MEDIAN_KM * (KAPPA_SIGMA * normal(rng)).exp();
    (beta, kappa)
}

/// β̂ MLE 重拟合子采样统计（每 100 条轨迹拟合一次）。
fn beta_fit_subsample(errs: &[f64]) -> Value {
    let mean = errs.iter().sum::<f64>() / errs.len() as f64;
    let max = errs.iter().cloned().fold(0.0, f64::max);
    json!({
        "n": errs.len(),
        "mean_abs_err": mean,
        "max_abs_err": max,
    })
}

/* ---------- 组 1：levy_iid（草案 §7.3.3 字面生成器） ---------- */

fn levy_iid_group(trajectories: usize) -> Value {
    // 小 κ 轨迹去重率高，放宽原始步数上限避免窗口不足。
    let cfg = SimConfig {
        raw_step_cap: 20_000,
        ..SimConfig::default()
    };
    let mut rng = StdRng::seed_from_u64(0xA11CE);
    let mut alphas = Vec::with_capacity(trajectories);
    let mut gs = Vec::with_capacity(trajectories);
    let mut dup_total = 0u64;
    let mut fit_errs = Vec::new();
    for i in 0..trajectories {
        let (beta, kappa) = draw_mobility(&mut rng);
        let path = levy_path(&mut rng, beta, kappa, WINDOW, &cfg);
        dup_total += u64::from(path.duplicates_dropped);
        let disp = displacements_from_cells(&path.cells).expect("valid cells");
        let a = psd_alpha(&disp).unwrap_or_else(|e| {
            panic!("levy_iid trajectory {i} (beta={beta}, kappa={kappa}): {e}")
        });
        alphas.push(a.alpha);
        gs.push(bridge_check(a.alpha, beta).expect("beta in range").g);
        if i % 100 == 0 {
            let fitted = fit(&disp, None).expect("fit on >=64 samples");
            fit_errs.push((fitted.beta - beta).abs());
        }
    }
    json!({
        "trajectories": trajectories,
        "generator": "draft §7.3.3 literal: i.i.d. truncated Levy steps, random heading, H3 res10, §4.1 dedup",
        "beta_draw": "U[1.50, 1.90]",
        "kappa_draw": format!("lognormal(median={KAPPA_MEDIAN_KM}km, sigma={KAPPA_SIGMA})"),
        "alpha": alpha_stats(&mut alphas),
        "bridge_g": g_stats(&mut gs),
        "duplicates_dropped_mean": dup_total as f64 / trajectories as f64,
        "beta_fit_subsample": beta_fit_subsample(&fit_errs),
    })
}

/* ---------- 组 2：trip_walk（GeoYuan 扩展） ---------- */

fn trip_group(trajectories: usize, trip: &TripConfig) -> Value {
    let cfg = SimConfig {
        raw_step_cap: 20_000,
        ..SimConfig::default()
    };
    let mut rng = StdRng::seed_from_u64(0xBEA57);
    let mut alphas = Vec::with_capacity(trajectories);
    let mut gs = Vec::with_capacity(trajectories);
    let mut dup_total = 0u64;
    let mut fit_errs = Vec::new();
    for i in 0..trajectories {
        let (beta, kappa) = draw_mobility(&mut rng);
        let path = trip_walk_path(&mut rng, beta, kappa, WINDOW, &cfg, trip);
        dup_total += u64::from(path.duplicates_dropped);
        let disp = displacements_from_cells(&path.cells).expect("valid cells");
        let a = psd_alpha(&disp).unwrap_or_else(|e| {
            panic!("trip_walk trajectory {i} (beta={beta}, kappa={kappa}): {e}")
        });
        alphas.push(a.alpha);
        gs.push(bridge_check(a.alpha, beta).expect("beta in range").g);
        if i % 100 == 0 {
            let fitted = fit(&disp, None).expect("fit on >=64 samples");
            fit_errs.push((fitted.beta - beta).abs());
        }
    }
    json!({
        "trajectories": trajectories,
        "generator": format!(
            "GeoYuan trip walk: Levy trip distance split into n~U{{1..{}}} correlated sub-steps, jitter σ={}, heading jitter {} rad",
            trip.max_substeps, trip.jitter_sigma, trip.heading_jitter_rad
        ),
        "beta_draw": "U[1.50, 1.90]",
        "kappa_draw": format!("lognormal(median={KAPPA_MEDIAN_KM}km, sigma={KAPPA_SIGMA})"),
        "alpha": alpha_stats(&mut alphas),
        "bridge_g": g_stats(&mut gs),
        "duplicates_dropped_mean": dup_total as f64 / trajectories as f64,
        "beta_fit_subsample": beta_fit_subsample(&fit_errs),
    })
}

/* ---------- 组 3：纯随机游走（白噪声位移，预期 α≈0） ---------- */

fn control_random_walk(trajectories: usize) -> Value {
    let mut rng = StdRng::seed_from_u64(0xB0B5EED);
    let mut alphas = Vec::with_capacity(trajectories);
    for _ in 0..trajectories {
        let disp: Vec<f64> = (0..WINDOW)
            .map(|_| 0.2 + 0.5 * normal(&mut rng).abs())
            .collect();
        alphas.push(psd_alpha(&disp).unwrap().alpha);
    }
    json!({
        "trajectories": trajectories,
        "generator": "i.i.d. |N(0,1)|·0.5 + 0.2 km",
        "alpha": alpha_stats(&mut alphas),
    })
}

/* ---------- 组 4：确定性重放 + 线性漂移（预期 α≈2） ---------- */

fn control_replay(trajectories: usize) -> Value {
    // 先用固定种子生成一条"被录制"的基准位移序列，所有重放共享它。
    let mut rng = StdRng::seed_from_u64(0xC0FFEE);
    let base_levy = LevyParams::new(1.75, 5.0, 0.1).unwrap();
    let base: Vec<f64> = (0..WINDOW).map(|_| base_levy.sample(&mut rng)).collect();
    let mut rng = StdRng::seed_from_u64(0x000F_E9A1_5EED);
    let mut alphas = Vec::with_capacity(trajectories);
    for _ in 0..trajectories {
        let disp: Vec<f64> = base
            .iter()
            .enumerate()
            .map(|(i, b)| b + REPLAY_DRIFT_KM * i as f64 + REPLAY_JITTER_KM * normal(&mut rng))
            .collect();
        alphas.push(psd_alpha(&disp).unwrap().alpha);
    }
    json!({
        "trajectories": trajectories,
        "generator": "shared recorded series + 0.05 km/step drift + 1e-4 km jitter",
        "alpha": alpha_stats(&mut alphas),
    })
}

/* ---------- 组 5：高斯相关随机游走（预期 α 在生物区间外） ---------- */

fn control_correlated(trajectories: usize) -> Value {
    let mut rng = StdRng::seed_from_u64(0xD00D5EED);
    let mut alphas = Vec::with_capacity(trajectories);
    for _ in 0..trajectories {
        // AR(1) 速度（φ=0.9）积分成位置 → 位移序列呈强低频相关。
        let mut v = 0.0;
        let mut pos = 0.0;
        let disp: Vec<f64> = (0..WINDOW)
            .map(|_| {
                v = 0.9 * v + 0.1 * normal(&mut rng);
                pos += v;
                pos.abs() + 1e-3
            })
            .collect();
        alphas.push(psd_alpha(&disp).unwrap().alpha);
    }
    json!({
        "trajectories": trajectories,
        "generator": "|∫ AR(1) velocity (phi=0.9)| + 1e-3 km",
        "alpha": alpha_stats(&mut alphas),
    })
}

/* ---------- --calibrate：trip 结构参数扫描 ---------- */

fn calibrate_main(output: Option<&str>) {
    let cfg = SimConfig {
        raw_step_cap: 20_000,
        ..SimConfig::default()
    };
    let grid_cfgs = [
        (1, 0.35), // 退化：n=1 即 i.i.d.（对照锚点）
        (4, 0.35),
        (6, 0.50),
        (8, 0.35),
        (8, 0.50),
        (8, 0.70),
        (12, 0.50),
        (12, 0.70),
        (16, 0.50),
        (16, 0.70),
        (24, 0.70),
        (24, 1.00),
    ];
    let mut grid = Vec::with_capacity(grid_cfgs.len());
    println!(
        "calibrating TripConfig (max_substeps, jitter_sigma): {} trajectories per point, window={WINDOW}",
        CALIBRATION_TRAJECTORIES
    );
    println!(
        "{:>5} {:>6}  {:>7} {:>7} {:>7} {:>8}  {:>7}",
        "n_max", "sigma", "p05", "p50", "p95", "frac_bio", "g_frac"
    );
    for (i, &(ms, js)) in grid_cfgs.iter().enumerate() {
        let trip = TripConfig {
            max_substeps: ms,
            jitter_sigma: js,
            heading_jitter_rad: 0.35,
        };
        let mut rng = StdRng::seed_from_u64(0xCA11_BA5E + i as u64);
        let mut alphas = Vec::with_capacity(CALIBRATION_TRAJECTORIES);
        let mut gs = Vec::with_capacity(CALIBRATION_TRAJECTORIES);
        for j in 0..CALIBRATION_TRAJECTORIES {
            let (beta, kappa) = draw_mobility(&mut rng);
            let path = trip_walk_path(&mut rng, beta, kappa, WINDOW, &cfg, &trip);
            let disp = displacements_from_cells(&path.cells).expect("valid cells");
            let a = psd_alpha(&disp)
                .unwrap_or_else(|e| panic!("calibrate n_max={ms} σ={js} traj {j}: {e}"));
            alphas.push(a.alpha);
            gs.push(bridge_check(a.alpha, beta).expect("beta in range").g);
        }
        let alpha = alpha_stats(&mut alphas);
        let g = g_stats(&mut gs);
        println!(
            "{ms:5} {js:6.2}  {:7.3} {:7.3} {:7.3} {:8.3}  {:7.3}",
            alpha["p05"].as_f64().unwrap(),
            alpha["p50"].as_f64().unwrap(),
            alpha["p95"].as_f64().unwrap(),
            alpha["frac_in_bio_030_080"].as_f64().unwrap(),
            g["frac_in_03_07"].as_f64().unwrap(),
        );
        grid.push(json!({
            "max_substeps": ms, "jitter_sigma": js,
            "alpha": alpha, "bridge_g": g,
        }));
    }
    let report = json!({
        "mode": "calibrate",
        "heading_jitter_rad": 0.35,
        "trajectories_per_point": CALIBRATION_TRAJECTORIES,
        "window_displacements": WINDOW,
        "target": "median alpha in [0.50, 0.60] (bridge g median ~0.44); maximize frac_bio & g_frac",
        "grid": grid,
    });
    let text = serde_json::to_string_pretty(&report).unwrap();
    match output {
        Some(path) => {
            std::fs::write(path, &text).expect("write calibration report");
            println!("calibration report written to {path}");
        }
        None => println!("{text}"),
    }
}

/* ---------- 全量验证 ---------- */

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let calibrate = args.iter().any(|a| a == "--calibrate");
    let output = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .map(String::as_str);
    if calibrate {
        calibrate_main(output);
        return;
    }

    let levy_iid = levy_iid_group(TRAJECTORIES);
    let trip_walk = trip_group(TRAJECTORIES, &TripConfig::default());
    let random_walk = control_random_walk(TRAJECTORIES);
    let replay = control_replay(TRAJECTORIES);
    let correlated = control_correlated(TRAJECTORIES);

    let f = |v: &Value, k1: &str, k2: &str| v[k1][k2].as_f64().unwrap();
    let iid_g_frac = f(&levy_iid, "bridge_g", "frac_in_03_07");
    let trip_g_frac = f(&trip_walk, "bridge_g", "frac_in_03_07");
    let trip_bio = f(&trip_walk, "alpha", "frac_in_bio_030_080");
    let control_bio = f(&random_walk, "alpha", "frac_in_bio_030_080")
        .max(f(&replay, "alpha", "frac_in_bio_030_080"))
        .max(f(&correlated, "alpha", "frac_in_bio_030_080"));

    let report = json!({
        "spec": "draft-ayerbe-trip-protocol-04 §7.3.3 Numerical Validation",
        "generated_by": "trip-core examples/monte_carlo (deterministic seeds)",
        "window_displacements": WINDOW,
        "scientific_note": "§7.3.3 literal generator (i.i.d. truncated Levy) yields a flat displacement spectrum (alpha~0) and fails the §7.3 bridge check; amplitude-envelope modulation is also nullified by the heavy-tailed noise and §4.1 dedup gating. GeoYuan's trip-structured walk (a Levy trip distance split into correlated sub-steps) restores the biological band. Trip-structured series also bias the i.i.d.-Levy MLE beta_hat upward (mean |err| ~1.15 vs true beta); alpha classification and the fitted 99.9% anomaly threshold remain usable. Per-trajectory alpha at window 256 is noisy (std ~0.3): band membership requires multi-window accumulation (W4 trust scoring), not single-shot rejection. Literal group reported honestly as upstream draft feedback.",
        "groups": {
            "levy_iid": levy_iid,
            "trip_walk": trip_walk,
            "random_walk": random_walk,
            "replay_with_drift": replay,
            "correlated_gaussian": correlated,
        },
        "conclusions": {
            "levy_iid_alpha_p50": f(&levy_iid, "alpha", "p50"),
            "levy_iid_bridge_g_frac_in_range": iid_g_frac,
            "literal_generator_fails_bridge": iid_g_frac < 0.5,
            "trip_walk_alpha_p50": f(&trip_walk, "alpha", "p50"),
            "trip_walk_bridge_g_frac_in_range": trip_g_frac,
            "trip_walk_frac_bio": trip_bio,
            "trip_walk_beta_fit_mean_abs_err": trip_walk["beta_fit_subsample"]["mean_abs_err"]
                .as_f64()
                .unwrap(),
            "control_max_frac_bio": control_bio,
            "trip_minus_control_frac_bio_gap": trip_bio - control_bio,
            // 分离判据：人类参考组的生物区间命中率须比最强对照组高 30 个
            // 百分点以上（对照组 <5%，trip 组 >35% 即可通过本判据）。
            "trip_separates_from_controls": trip_bio > control_bio + 0.30,
        },
    });
    let text = serde_json::to_string_pretty(&report).unwrap();
    match output {
        Some(path) => {
            std::fs::write(path, &text).expect("write report");
            println!("report written to {path}");
        }
        None => println!("{text}"),
    }
}
