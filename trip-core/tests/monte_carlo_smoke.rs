//! Monte Carlo smoke 测试：在 500 条轨迹上验证五组生成器的统计边界。
//!
//! 全量 10,000 条 MC 见 `examples/monte_carlo.rs`（release 约 20–40 秒），
//! 报告存 `tests/vectors/monte-carlo-report.json`。本 smoke 用更小的样本
//! 快速验证边界方向性，阈值取自校准与全量 MC 的保守余量。

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use trip_core::engine::levy::{bridge_check, fit};
use trip_core::engine::psd::{displacements_from_cells, psd_alpha};
use trip_core::engine::sim::{levy_path, normal, trip_walk_path, SimConfig, TripConfig};

const SMOKE_TRAJECTORIES: usize = 500;
const WINDOW: usize = 256;
const KAPPA_MEDIAN_KM: f64 = 5.0;
const KAPPA_SIGMA: f64 = 1.0;
const REPLAY_DRIFT_KM: f64 = 0.05;
const REPLAY_JITTER_KM: f64 = 1e-4;

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

fn frac_bio(alphas: &[f64]) -> f64 {
    alphas.iter().filter(|a| (0.30..=0.80).contains(*a)).count() as f64 / alphas.len() as f64
}

fn frac_g_in_range(gs: &[f64]) -> f64 {
    gs.iter().filter(|g| (0.3..=0.7).contains(*g)).count() as f64 / gs.len() as f64
}

fn draw_beta_kappa(rng: &mut StdRng) -> (f64, f64) {
    let beta = 1.50 + 0.40 * rng.gen::<f64>();
    let kappa = KAPPA_MEDIAN_KM * (KAPPA_SIGMA * normal(rng)).exp();
    (beta, kappa)
}

#[test]
fn smoke_literal_levy_fails_bridge() {
    // 草案 §7.3.3 字面生成器：i.i.d. 截断 Levy → α≈0，桥校验必然失败。
    // 这是给上游 rats@ietf.org 的 draft feedback，如实固化。
    let cfg = SimConfig {
        raw_step_cap: 20_000,
        ..SimConfig::default()
    };
    let mut rng = StdRng::seed_from_u64(0xA11CE);
    let mut alphas = Vec::with_capacity(SMOKE_TRAJECTORIES);
    let mut gs = Vec::with_capacity(SMOKE_TRAJECTORIES);
    for _ in 0..SMOKE_TRAJECTORIES {
        let (beta, kappa) = draw_beta_kappa(&mut rng);
        let path = levy_path(&mut rng, beta, kappa, WINDOW, &cfg);
        let disp = displacements_from_cells(&path.cells).unwrap();
        let a = psd_alpha(&disp).unwrap();
        alphas.push(a.alpha);
        gs.push(bridge_check(a.alpha, beta).unwrap().g);
    }
    alphas.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let med = percentile(&alphas, 0.50);
    let frac_bio = frac_bio(&alphas);
    let frac_g = frac_g_in_range(&gs);
    // i.i.d. 位移谱平坦：中位 α 应远离生物区间下端 0.30。
    assert!(
        med < 0.15,
        "iid levy median alpha should be ~0 (flat), got {med}"
    );
    assert!(
        frac_bio < 0.10,
        "iid levy should rarely fall in biological band, got {frac_bio}"
    );
    assert!(
        frac_g < 0.10,
        "iid levy should fail the bridge check, got g_frac={frac_g}"
    );
}

#[test]
fn smoke_trip_walk_hits_biological_band() {
    // GeoYuan 扩展：trip 结构化 Levy（默认参数 max_substeps=16, σ=0.50）。
    // 校准目标：中位 α ≈ 0.55，frac_bio > 0.50，frac_g > 0.50。
    let cfg = SimConfig {
        raw_step_cap: 20_000,
        ..SimConfig::default()
    };
    let trip = TripConfig::default();
    let mut rng = StdRng::seed_from_u64(0xBEA57);
    let mut alphas = Vec::with_capacity(SMOKE_TRAJECTORIES);
    let mut gs = Vec::with_capacity(SMOKE_TRAJECTORIES);
    let mut fit_errs = Vec::new();
    for i in 0..SMOKE_TRAJECTORIES {
        let (beta, kappa) = draw_beta_kappa(&mut rng);
        let path = trip_walk_path(&mut rng, beta, kappa, WINDOW, &cfg, &trip);
        let disp = displacements_from_cells(&path.cells).unwrap();
        let a = psd_alpha(&disp).unwrap();
        alphas.push(a.alpha);
        gs.push(bridge_check(a.alpha, beta).unwrap().g);
        if i % 50 == 0 {
            let f = fit(&disp, None).unwrap();
            fit_errs.push((f.beta - beta).abs());
        }
    }
    alphas.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let med = percentile(&alphas, 0.50);
    let fb = frac_bio(&alphas);
    let fg = frac_g_in_range(&gs);
    assert!(
        (0.35..=0.75).contains(&med),
        "trip_walk median alpha should be in [0.35, 0.75], got {med}"
    );
    assert!(fb > 0.35, "trip_walk frac_bio should be > 0.35, got {fb}");
    assert!(
        fg > 0.30,
        "trip_walk bridge g frac should be > 0.30, got {fg}"
    );
    // 重尾边际上 MLE 仍可用（产出有限异常阈值）。注意：trip 结构化序列
    // 的 β̂ 有系统性上偏（MLE 假设 i.i.d.，而子步幅度相关）——β̂ 恢复
    // 真值的保证由 engine_golden 的 i.i.d. levy 向量覆盖，不适用于本组。
    let mean_err = fit_errs.iter().sum::<f64>() / fit_errs.len() as f64;
    assert!(
        mean_err.is_finite() && mean_err >= 0.0,
        "trip_walk beta fit should succeed with finite errors, got {mean_err}"
    );
}

#[test]
fn smoke_random_walk_is_not_biological() {
    // 纯白噪声位移：预期 α≈0，frac_bio 远低于 trip_walk。
    let mut rng = StdRng::seed_from_u64(0xB0B5EED);
    let mut alphas = Vec::with_capacity(SMOKE_TRAJECTORIES);
    for _ in 0..SMOKE_TRAJECTORIES {
        let disp: Vec<f64> = (0..WINDOW)
            .map(|_| 0.2 + 0.5 * normal(&mut rng).abs())
            .collect();
        alphas.push(psd_alpha(&disp).unwrap().alpha);
    }
    let fb = frac_bio(&alphas);
    assert!(
        fb < 0.10,
        "random walk should rarely fall in biological band, got {fb}"
    );
}

#[test]
fn smoke_replay_with_drift_is_not_biological() {
    // 共享录制序列 + 线性漂移：预期 α≈2（趋势谱），远离生物区间。
    use trip_core::engine::levy::LevyParams;
    let mut rng = StdRng::seed_from_u64(0xC0FFEE);
    let base_levy = LevyParams::new(1.75, 5.0, 0.1).unwrap();
    let base: Vec<f64> = (0..WINDOW).map(|_| base_levy.sample(&mut rng)).collect();
    let mut rng = StdRng::seed_from_u64(0x000F_E9A1_5EED);
    let mut alphas = Vec::with_capacity(SMOKE_TRAJECTORIES);
    for _ in 0..SMOKE_TRAJECTORIES {
        let disp: Vec<f64> = base
            .iter()
            .enumerate()
            .map(|(i, b)| b + REPLAY_DRIFT_KM * i as f64 + REPLAY_JITTER_KM * normal(&mut rng))
            .collect();
        alphas.push(psd_alpha(&disp).unwrap().alpha);
    }
    let fb = frac_bio(&alphas);
    assert!(
        fb < 0.10,
        "replay with drift should rarely fall in biological band, got {fb}"
    );
}

#[test]
fn smoke_correlated_gaussian_is_not_biological() {
    // AR(1) 速度积分：强低频相关，α 偏高（> 1.0），远离生物区间。
    let mut rng = StdRng::seed_from_u64(0xD00D5EED);
    let mut alphas = Vec::with_capacity(SMOKE_TRAJECTORIES);
    for _ in 0..SMOKE_TRAJECTORIES {
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
    let fb = frac_bio(&alphas);
    assert!(
        fb < 0.20,
        "correlated gaussian should rarely fall in biological band, got {fb}"
    );
}

#[test]
fn smoke_trip_walk_separates_from_all_controls() {
    // 边界可分性：trip_walk 的 frac_bio 必须显著高于所有对照组。
    let cfg = SimConfig {
        raw_step_cap: 20_000,
        ..SimConfig::default()
    };
    let trip = TripConfig::default();
    let mut rng = StdRng::seed_from_u64(0xBEA57);
    let mut trip_alphas = Vec::with_capacity(SMOKE_TRAJECTORIES);
    for _ in 0..SMOKE_TRAJECTORIES {
        let (beta, kappa) = draw_beta_kappa(&mut rng);
        let path = trip_walk_path(&mut rng, beta, kappa, WINDOW, &cfg, &trip);
        let disp = displacements_from_cells(&path.cells).unwrap();
        trip_alphas.push(psd_alpha(&disp).unwrap().alpha);
    }
    let trip_bio = frac_bio(&trip_alphas);

    // 对照组直接用 i.i.d. Levy（最接近 trip_walk 的对照）。
    let mut rng = StdRng::seed_from_u64(0xA11CE);
    let mut iid_alphas = Vec::with_capacity(SMOKE_TRAJECTORIES);
    for _ in 0..SMOKE_TRAJECTORIES {
        let (beta, kappa) = draw_beta_kappa(&mut rng);
        let path = levy_path(&mut rng, beta, kappa, WINDOW, &cfg);
        let disp = displacements_from_cells(&path.cells).unwrap();
        iid_alphas.push(psd_alpha(&disp).unwrap().alpha);
    }
    let iid_bio = frac_bio(&iid_alphas);

    assert!(
        trip_bio > iid_bio + 0.30,
        "trip_walk ({trip_bio}) should dominate iid levy ({iid_bio}) by > 0.30"
    );
}
