//! 引擎黄金测试向量生成器（PSD / Levy MLE / H3 位移）。
//!
//! 运行：
//!
//! ```bash
//! cargo run -p trip-core --example gen_engine_vectors > trip-core/tests/vectors/engine-vectors.json
//! ```
//!
//! 与协议向量不同，引擎向量使用**可跨语言复现的 xorshift64\*** PRNG
//! （而非 ChaCha12），任何语言按相同步骤即可重建输入序列并校验输出。
//! 浮点结果同时给出 IEEE-754 位模式（hex）与 17 位十进制；跨实现校验
//! 目标：α/R²/置信度绝对偏差 < 1e-9，β̂/κ̂ 相对偏差 < 1e-6。
//!
//! 覆盖四类信号 + 一个 H3 cell 用例：
//!
//! | 向量 | 生成器 | 预期 |
//! |---|---|---|
//! | white | `1 + U(0,1)` i.i.d. | α ≈ 0（White） |
//! | brown | `100 + cumsum(U(0,1)−0.5)` | α ≈ 2（Brown） |
//! | pink_voss | Voss-McCartney 9 行 | α ≈ 1（近协议中心） |
//! | levy_sample | 截断 Levy β=1.75 κ=5 r_min=0.1 | MLE β̂≈1.75、κ̂≈5 |
//! | cells_case | Levy 游走 → res10 量化 → §4.1 去重 | 65 cells → α |

use serde_json::{json, Value};
use trip_core::engine::levy::{bridge_check, fit, LevyParams};
use trip_core::engine::psd::{displacements_from_cells, psd_alpha};
use trip_core::engine::sim::{destination, SimConfig};

const N: usize = 256;

/// xorshift64*：见 Marsaglia (2003) / Vigna (2014)。种子非零。
struct XorShift(u64);

impl XorShift {
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

/// f64 的位模式 hex（8 字节大端语义即 to_bits）。
fn bits(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn bits_list(series: &[f64]) -> Vec<String> {
    series.iter().map(|v| bits(*v)).collect()
}

fn expected_json(a: trip_core::engine::psd::PsdAnalysis) -> Value {
    json!({
        "alpha": a.alpha,
        "alpha_bits_hex": bits(a.alpha),
        "r_squared": a.r_squared,
        "r_squared_bits_hex": bits(a.r_squared),
        "confidence": a.confidence,
        "confidence_bits_hex": bits(a.confidence),
        "classification": a.classification.as_str(),
    })
}

fn main() {
    // 向量专用种子（公开测试夹具，勿在生产中使用）。
    let seed: u64 = 0x5EED_2026_0916_0001;

    /* ---------- white ---------- */
    let mut rng = XorShift(seed);
    let white: Vec<f64> = (0..N).map(|_| 1.0 + rng.uniform()).collect();

    /* ---------- brown ---------- */
    let mut rng = XorShift(seed ^ 0x000B_7074);
    let mut acc = 100.0f64;
    let brown: Vec<f64> = (0..N)
        .map(|_| {
            acc += 2.0 * rng.uniform() - 1.0;
            acc
        })
        .collect();

    /* ---------- pink_voss ---------- */
    let mut rng = XorShift(seed ^ 0x91A9);
    const ROWS: usize = 9;
    let mut rows = [0.0f64; ROWS];
    let pink: Vec<f64> = (0..N)
        .map(|i| {
            for (k, slot) in rows.iter_mut().enumerate() {
                if i % (1usize << k) == 0 {
                    *slot = rng.uniform();
                }
            }
            rows.iter().sum::<f64>()
        })
        .collect();

    /* ---------- levy_sample（含 MLE 拟合期望值） ---------- */
    let levy_params = LevyParams::new(1.75, 5.0, 0.1).unwrap();
    let mut rng = XorShift(seed ^ 0x13E7);
    let levy: Vec<f64> = (0..N)
        .map(|_| loop {
            let u1 = rng.uniform();
            let u2 = rng.uniform();
            if let Some(r) = levy_params.sample_from_uniforms(u1, u2) {
                break r;
            }
        })
        .collect();

    /* ---------- cells_case ---------- */
    // 从北京出发的截断 Levy 游走（β=1.7, κ=3.0），res10 量化 + §4.1 去重，
    // 收集 65 个 cell（= 64 个位移，DFT 最小窗口）。
    let walk_params = LevyParams::new(1.7, 3.0, 0.1).unwrap();
    let cfg = SimConfig::default();
    let mut rng = XorShift(seed ^ 0xCE11);
    let (mut lat, mut lng) = (
        cfg.start_lat_deg.to_radians(),
        cfg.start_lng_deg.to_radians(),
    );
    let first = h3o::LatLng::new(lat.to_degrees(), lng.to_degrees())
        .unwrap()
        .to_cell(cfg.resolution);
    let mut cells = vec![u64::from(first)];
    while cells.len() < 65 {
        let r = loop {
            let u1 = rng.uniform();
            let u2 = rng.uniform();
            if let Some(r) = walk_params.sample_from_uniforms(u1, u2) {
                break r;
            }
        };
        let bearing = TAU * rng.uniform();
        let (lat2, lng2) = destination(lat, lng, bearing, r);
        let cell = h3o::LatLng::new(lat2.to_degrees(), lng2.to_degrees())
            .unwrap()
            .to_cell(cfg.resolution);
        let raw = u64::from(cell);
        if raw != *cells.last().unwrap() {
            cells.push(raw);
            lat = lat2;
            lng = lng2;
        }
    }

    /* ---------- 计算期望值 ---------- */
    let a_white = psd_alpha(&white).unwrap();
    let a_brown = psd_alpha(&brown).unwrap();
    let a_pink = psd_alpha(&pink).unwrap();
    let a_levy = psd_alpha(&levy).unwrap();
    let f_levy = fit(&levy, None).unwrap();
    let g_levy = bridge_check(a_levy.alpha, f_levy.beta).unwrap();
    let cell_disp = displacements_from_cells(&cells).unwrap();
    let a_cells = psd_alpha(&cell_disp).unwrap();

    let report = json!({
        "spec": "draft-ayerbe-trip-protocol-04 §7 (Criticality Engine)",
        "generated_by": "trip-core examples/gen_engine_vectors (deterministic; do not edit by hand)",
        "prng": {
            "algorithm": "xorshift64* (Marsaglia/Vigna), multiplier 0x2545F4914F6CDD1D",
            "seed_hex": format!("{seed:016x}"),
            "uniform": "(x >> 11) / 2^53",
            "series_seeds_hex": {
                "white": format!("{:016x}", seed),
                "brown": format!("{:016x}", seed ^ 0xB7074),
                "pink_voss": format!("{:016x}", seed ^ 0x91A9),
                "levy_sample": format!("{:016x}", seed ^ 0x13E7),
                "cells_case": format!("{:016x}", seed ^ 0xCE11),
            }
        },
        "algorithm_notes": {
            "psd": "naive DFT k=1..=N/2, angle = -2π·((j·k) mod N)/N; OLS on (ln f, ln S) with f=k/N; alpha=-slope; confidence=max(0,1-|α-0.55|/0.25)·R²",
            "levy_fit": "deterministic grid MLE: beta ∈ [1.0,3.0] step 0.02; kappa log-grid 40 pts in [0.05,500] + two local refinements; K(u;β) via Simpson 512 nodes on s-substitution, TAIL_SPAN=40; r_min = min(sample)",
            "levy_sampling": "Pareto propose r = r_min·u1^(-1/(β-1)), accept iff u2 < exp(-r/κ); one uniform pair per attempt",
            "tolerances": "cross-implementation: |Δα|,|ΔR²|,|Δconf| < 1e-9; rel Δβ̂, Δκ̂ < 1e-6"
        },
        "window": N,
        "series": {
            "white": {
                "generator": "1.0 + U(0,1), i.i.d.",
                "values_bits_hex": bits_list(&white),
                "expected": expected_json(a_white),
            },
            "brown": {
                "generator": "100 + cumsum(2·U(0,1) − 1)",
                "values_bits_hex": bits_list(&brown),
                "expected": expected_json(a_brown),
            },
            "pink_voss": {
                "generator": "Voss-McCartney, 9 octave rows, row k refreshed when i mod 2^k == 0",
                "values_bits_hex": bits_list(&pink),
                "expected": expected_json(a_pink),
            },
            "levy_sample": {
                "params": { "beta": 1.75, "kappa": 5.0, "r_min": 0.1 },
                "values_bits_hex": bits_list(&levy),
                "expected": expected_json(a_levy),
                "fit": {
                    "beta_hat": f_levy.beta,
                    "beta_hat_bits_hex": bits(f_levy.beta),
                    "kappa_hat": f_levy.kappa,
                    "kappa_hat_bits_hex": bits(f_levy.kappa),
                    "r_min_used": f_levy.r_min,
                    "log_likelihood": f_levy.log_likelihood,
                    "threshold_999": f_levy.threshold_999,
                    "threshold_999_bits_hex": bits(f_levy.threshold_999),
                    "bridge_g": g_levy.g,
                    "bridge_consistent": g_levy.consistent,
                }
            }
        },
        "cells_case": {
            "resolution": 10,
            "walk": { "beta": 1.7, "kappa": 3.0, "r_min": 0.1 },
            "start": { "lat_deg": 39.9042, "lng_deg": 116.4074 },
            "cells_u64": cells,
            "expected": expected_json(a_cells),
        }
    });

    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

const TAU: f64 = core::f64::consts::TAU;
