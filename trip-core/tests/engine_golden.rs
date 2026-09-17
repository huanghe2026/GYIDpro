//! 引擎黄金向量重放测试（PSD / Levy MLE / H3 位移）。
//!
//! 完整重放 `tests/vectors/engine-vectors.json` 的生成管线：
//! 1. 用向量中公开的 xorshift64\* 种子**重建**输入序列，与 JSON 中的
//!    `values_bits_hex` 逐位比对（验证 PRNG 跨实现规格）；
//! 2. 对重建序列运行引擎（`psd_alpha` / `fit` / `bridge_check` /
//!    `displacements_from_cells`），与 `expected` 比对：
//!    |Δα|、|ΔR²|、|Δconf| < 1e-9；|Δβ̂|、|Δκ̂| 相对偏差 < 1e-6。
//!
//! 这是 W4/trip-server 等其他实现（含非 Rust 实现）的互操作基准。

use serde_json::Value;
use trip_core::engine::levy::{bridge_check, fit, LevyParams};
use trip_core::engine::psd::{displacements_from_cells, psd_alpha};
use trip_core::engine::sim::destination;

const TOLERANCE: f64 = 1e-9;
const REL_TOLERANCE: f64 = 1e-6;

fn load_vectors() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/vectors/engine-vectors.json"
    );
    let text = std::fs::read_to_string(path).expect("engine-vectors.json exists");
    serde_json::from_str(&text).expect("valid JSON")
}

/// xorshift64*（Marsaglia/Vigna），与生成器逐位一致。
struct XorShift(u64);

impl XorShift {
    fn new(seed_hex: &str) -> Self {
        Self(u64::from_str_radix(seed_hex, 16).expect("hex seed"))
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// 均匀 (0,1)：取高 53 位。
    fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) / (1u64 << 53) as f64
    }
}

fn bits_to_f64(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).expect("hex bits"))
}

fn json_bits_list(v: &Value) -> Vec<f64> {
    v.as_array()
        .expect("bits array")
        .iter()
        .map(|s| bits_to_f64(s.as_str().expect("hex string")))
        .collect()
}

/// 重建序列并与 JSON 位模式逐位比对（PRNG 跨实现规格验证）。
fn assert_series_reproduces(json_series: &Value, rebuilt: &[f64]) {
    let recorded = json_bits_list(&json_series["values_bits_hex"]);
    assert_eq!(
        rebuilt.len(),
        recorded.len(),
        "series length mismatch for {}",
        json_series["generator"].as_str().unwrap_or("?")
    );
    for (i, (a, b)) in rebuilt.iter().zip(recorded.iter()).enumerate() {
        assert_eq!(a.to_bits(), b.to_bits(), "series bit mismatch at index {i}");
    }
}

fn assert_expected(json_series: &Value, alpha: f64, r_squared: f64, confidence: f64, class: &str) {
    let exp = &json_series["expected"];
    assert!(
        (alpha - exp["alpha"].as_f64().unwrap()).abs() < TOLERANCE,
        "alpha mismatch: got {alpha}, want {}",
        exp["alpha"]
    );
    assert!(
        (r_squared - exp["r_squared"].as_f64().unwrap()).abs() < TOLERANCE,
        "r_squared mismatch: got {r_squared}, want {}",
        exp["r_squared"]
    );
    assert!(
        (confidence - exp["confidence"].as_f64().unwrap()).abs() < TOLERANCE,
        "confidence mismatch: got {confidence}, want {}",
        exp["confidence"]
    );
    assert_eq!(class, exp["classification"].as_str().unwrap());
}

/* ---------- 序列重建（与 examples/gen_engine_vectors.rs 相同步骤） ---------- */

fn rebuild_white(seed: u64, n: usize) -> Vec<f64> {
    let mut rng = XorShift::new(&format!("{seed:016x}"));
    (0..n).map(|_| 1.0 + rng.uniform()).collect()
}

fn rebuild_brown(seed: u64, n: usize) -> Vec<f64> {
    let mut rng = XorShift::new(&format!("{:016x}", seed ^ 0xB7074));
    let mut acc = 100.0f64;
    (0..n)
        .map(|_| {
            acc += 2.0 * rng.uniform() - 1.0;
            acc
        })
        .collect()
}

fn rebuild_pink_voss(seed: u64, n: usize) -> Vec<f64> {
    let mut rng = XorShift::new(&format!("{:016x}", seed ^ 0x91A9));
    const ROWS: usize = 9;
    let mut rows = [0.0f64; ROWS];
    (0..n)
        .map(|i| {
            for (k, slot) in rows.iter_mut().enumerate() {
                if i % (1usize << k) == 0 {
                    *slot = rng.uniform();
                }
            }
            rows.iter().sum::<f64>()
        })
        .collect()
}

fn rebuild_levy(seed: u64, beta: f64, kappa: f64, r_min: f64, n: usize) -> Vec<f64> {
    let levy = LevyParams::new(beta, kappa, r_min).unwrap();
    let mut rng = XorShift::new(&format!("{:016x}", seed ^ 0x13E7));
    (0..n)
        .map(|_| loop {
            let u1 = rng.uniform();
            let u2 = rng.uniform();
            if let Some(r) = levy.sample_from_uniforms(u1, u2) {
                break r;
            }
        })
        .collect()
}

fn rebuild_cells(
    seed: u64,
    beta: f64,
    kappa: f64,
    r_min: f64,
    resolution: u8,
    needed: usize,
) -> Vec<u64> {
    use h3o::Resolution;
    let walk = LevyParams::new(beta, kappa, r_min).unwrap();
    let mut rng = XorShift::new(&format!("{:016x}", seed ^ 0xCE11));
    let (mut lat, mut lng) = (39.9042_f64.to_radians(), 116.4074_f64.to_radians());
    let res = Resolution::try_from(resolution).expect("res 10");
    let first = h3o::LatLng::new(lat.to_degrees(), lng.to_degrees())
        .unwrap()
        .to_cell(res);
    let mut cells = vec![u64::from(first)];
    let tau = core::f64::consts::TAU;
    while cells.len() < needed {
        let r = loop {
            let u1 = rng.uniform();
            let u2 = rng.uniform();
            if let Some(r) = walk.sample_from_uniforms(u1, u2) {
                break r;
            }
        };
        let bearing = tau * rng.uniform();
        let (lat2, lng2) = destination(lat, lng, bearing, r);
        let cell = h3o::LatLng::new(lat2.to_degrees(), lng2.to_degrees())
            .unwrap()
            .to_cell(res);
        let raw = u64::from(cell);
        if raw != *cells.last().unwrap() {
            cells.push(raw);
            lat = lat2;
            lng = lng2;
        }
    }
    cells
}

/* ---------- 测试 ---------- */

#[test]
fn golden_white_noise() {
    let v = load_vectors();
    let seed = u64::from_str_radix(v["prng"]["seed_hex"].as_str().unwrap(), 16).unwrap();
    let n = v["window"].as_u64().unwrap() as usize;
    let series = rebuild_white(seed, n);
    assert_series_reproduces(&v["series"]["white"], &series);
    let a = psd_alpha(&series).unwrap();
    assert_expected(
        &v["series"]["white"],
        a.alpha,
        a.r_squared,
        a.confidence,
        a.classification.as_str(),
    );
}

#[test]
fn golden_brown_noise() {
    let v = load_vectors();
    let seed = u64::from_str_radix(v["prng"]["seed_hex"].as_str().unwrap(), 16).unwrap();
    let n = v["window"].as_u64().unwrap() as usize;
    let series = rebuild_brown(seed, n);
    assert_series_reproduces(&v["series"]["brown"], &series);
    let a = psd_alpha(&series).unwrap();
    assert_expected(
        &v["series"]["brown"],
        a.alpha,
        a.r_squared,
        a.confidence,
        a.classification.as_str(),
    );
}

#[test]
fn golden_pink_voss() {
    let v = load_vectors();
    let seed = u64::from_str_radix(v["prng"]["seed_hex"].as_str().unwrap(), 16).unwrap();
    let n = v["window"].as_u64().unwrap() as usize;
    let series = rebuild_pink_voss(seed, n);
    assert_series_reproduces(&v["series"]["pink_voss"], &series);
    let a = psd_alpha(&series).unwrap();
    assert_expected(
        &v["series"]["pink_voss"],
        a.alpha,
        a.r_squared,
        a.confidence,
        a.classification.as_str(),
    );
}

#[test]
fn golden_levy_sample_and_fit() {
    let v = load_vectors();
    let seed = u64::from_str_radix(v["prng"]["seed_hex"].as_str().unwrap(), 16).unwrap();
    let n = v["window"].as_u64().unwrap() as usize;
    let params = &v["series"]["levy_sample"]["params"];
    let beta = params["beta"].as_f64().unwrap();
    let kappa = params["kappa"].as_f64().unwrap();
    let r_min = params["r_min"].as_f64().unwrap();

    let series = rebuild_levy(seed, beta, kappa, r_min, n);
    assert_series_reproduces(&v["series"]["levy_sample"], &series);

    let a = psd_alpha(&series).unwrap();
    assert_expected(
        &v["series"]["levy_sample"],
        a.alpha,
        a.r_squared,
        a.confidence,
        a.classification.as_str(),
    );

    // MLE 拟合：i.i.d. 截断 Levy 样本上 β̂/κ̂ 应恢复真值（网格分辨率内）。
    let f = fit(&series, None).unwrap();
    let fit_exp = &v["series"]["levy_sample"]["fit"];
    let rel = |got: f64, want: f64| (got - want).abs() / want.abs();
    assert!(
        rel(f.beta, fit_exp["beta_hat"].as_f64().unwrap()) < REL_TOLERANCE,
        "beta_hat mismatch: got {}, want {}",
        f.beta,
        fit_exp["beta_hat"]
    );
    assert!(
        rel(f.kappa, fit_exp["kappa_hat"].as_f64().unwrap()) < REL_TOLERANCE,
        "kappa_hat mismatch: got {}, want {}",
        f.kappa,
        fit_exp["kappa_hat"]
    );
    assert!(
        rel(f.threshold_999, fit_exp["threshold_999"].as_f64().unwrap()) < REL_TOLERANCE,
        "threshold_999 mismatch"
    );
    assert_eq!(
        f.r_min,
        series.iter().cloned().fold(f64::INFINITY, f64::min)
    );

    // 桥校验：i.i.d. Levy 的 α≈0 ⇒ g 不落入 [0.3,0.7]（草案缺口，如实固化）。
    let g = bridge_check(a.alpha, f.beta).unwrap();
    let want_g = fit_exp["bridge_g"].as_f64().unwrap();
    assert!(
        (g.g - want_g).abs() < TOLERANCE,
        "bridge_g mismatch: got {}, want {want_g}",
        g.g
    );
    assert_eq!(
        g.consistent,
        fit_exp["bridge_consistent"].as_bool().unwrap(),
        "bridge consistency flag mismatch"
    );
    assert!(
        !g.consistent,
        "iid levy must fail the bridge check (draft gap)"
    );
}

#[test]
fn golden_h3_cells_case() {
    let v = load_vectors();
    let seed = u64::from_str_radix(v["prng"]["seed_hex"].as_str().unwrap(), 16).unwrap();
    let case = &v["cells_case"];
    let needed = case["cells_u64"].as_array().unwrap().len();

    let cells = rebuild_cells(
        seed,
        case["walk"]["beta"].as_f64().unwrap(),
        case["walk"]["kappa"].as_f64().unwrap(),
        case["walk"]["r_min"].as_f64().unwrap(),
        case["resolution"].as_u64().unwrap() as u8,
        needed,
    );
    let recorded: Vec<u64> = case["cells_u64"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap())
        .collect();
    assert_eq!(
        cells, recorded,
        "reconstructed H3 cells diverge from vector"
    );

    // §4.1 去重自检 + 位移谱。
    for w in cells.windows(2) {
        assert_ne!(w[0], w[1], "consecutive duplicate cell in vector");
    }
    let disp = displacements_from_cells(&cells).unwrap();
    let a = psd_alpha(&disp).unwrap();
    assert_expected(
        case,
        a.alpha,
        a.r_squared,
        a.confidence,
        a.classification.as_str(),
    );
}
