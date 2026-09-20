//! 测试链种子生成器（开发 / 测试夹具，不参与协议主流程）。
//!
//! 引擎（trip-core PSD）对位移序列有谱形要求：α ∈ [0.30, 0.80] 且
//! confidence = (1-|α-0.55|/0.25)·R² > 策略门槛。真实 15 分钟级 GPS 轨迹
//! 天然满足；但手工造测试数据时，白噪声步长的 α≈0 会被判 REJECTED，
//! 且 H3 res-10 量化噪声（cell 边长 ≈76m）会显著污染小尺度合成信号。
//!
//! 本工具用**引擎同款函数**闭环重试：频域合成 1/f^0.55 位移序列 →
//! 真实 h3o 量化 → `displacements_from_cells` + `psd_alpha` 预检 →
//! 通过后才签名出链，保证上传后 Verifier 引擎能算出合规结果。
//!
//! 用法：
//! ```text
//! cargo run -p gyid-shared --example seed_chain -- \
//!   --seed <hex64> --n 256 --gap 902 --start-ts <unix> \
//!   --start-lat 40.1042 --start-lng 116.6074 \
//!   --start-index 0 --prev-hash none|<hex64> --out /tmp/evidence.bin
//! ```
//! 输出文件 = 逐条面包屑 CBOR 顺序拼接，可直接作为
//! `POST /v1/evidence` 的 octet-stream body。

use std::fs::File;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use gyid_shared::breadcrumb::collect_breadcrumb;
use gyid_shared::identity::Identity;
use trip_core::engine::psd::{displacements_from_cells, psd_alpha};

const RES: u8 = 10;
const ALPHA_TARGET: f64 = 0.55;
/// 引擎预检门槛（比服务端策略 min_confidence=0.1 更严，留余量）
const ALPHA_LO: f64 = 0.50;
const ALPHA_HI: f64 = 0.62;
const MIN_CONF: f64 = 0.30;

/// 简易 LCG（测试夹具无需密码学随机）
struct Lcg(u64);

impl Lcg {
    fn from_time() -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock");
        Lcg(now.as_nanos() as u64 | 1)
    }
    fn next_f64(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// 频域合成 1/f^alpha 实序列（Hermitian 对称 + 随机相位），加均值基底保正。
/// 常数均值只落在 DC 分量（引擎跳过 k=0），不影响谱斜率。
fn synth_displacements(n: usize, alpha: f64, rng: &mut Lcg) -> Vec<f64> {
    let half = n / 2;
    let mut amp = vec![0.0f64; half];
    let mut phi = vec![0.0f64; half];
    for k in 1..=half {
        amp[k - 1] = (k as f64 / n as f64).powf(-alpha / 2.0);
        phi[k - 1] = rng.next_f64() * std::f64::consts::TAU;
    }
    let mut x = vec![0.0f64; n];
    for j in 0..n {
        let mut v = 0.0;
        for k in 1..=half {
            let ang = std::f64::consts::TAU * (j * k) as f64 / n as f64 + phi[k - 1];
            v += 2.0 * amp[k - 1] * ang.cos();
        }
        x[j] = v;
    }
    let mean = x.iter().sum::<f64>() / n as f64;
    let var = x.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let std = var.sqrt();
    // 均值 1km、std 0.25km：尺度须远大于 res-10 量化噪声（~60m），
    // 否则噪声地板在高频段压平谱线，α 被拉向 0
    x.iter()
        .map(|v| (1.0 + 0.25 * (v - mean) / std).clamp(0.1, 3.5))
        .collect()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut seed_hex = String::new();
    let mut n = 256usize;
    let mut gap: u64 = 902;
    let mut start_ts: u64 = 0;
    let mut lat: f64 = 40.1042;
    let mut lng: f64 = 116.6074;
    let mut start_index: u64 = 0;
    let mut prev_hash_hex = String::from("none");
    let mut out = String::from("/tmp/evidence.bin");
    while let Some(a) = args.next() {
        match a.as_str() {
            "--seed" => seed_hex = args.next().expect("seed"),
            "--n" => n = args.next().expect("n").parse().expect("n"),
            "--gap" => gap = args.next().expect("gap").parse().expect("gap"),
            "--start-ts" => start_ts = args.next().expect("ts").parse().expect("ts"),
            "--start-lat" => lat = args.next().expect("lat").parse().expect("lat"),
            "--start-lng" => lng = args.next().expect("lng").parse().expect("lng"),
            "--start-index" => start_index = args.next().expect("idx").parse().expect("idx"),
            "--prev-hash" => prev_hash_hex = args.next().expect("hash"),
            "--out" => out = args.next().expect("out"),
            other => panic!("unknown arg: {other}"),
        }
    }
    let identity = Identity::from_seed_hex(&seed_hex).expect("seed");
    let prev_hash = if prev_hash_hex == "none" {
        None
    } else {
        let bytes = hex::decode(prev_hash_hex.trim()).expect("prev hash hex");
        assert_eq!(bytes.len(), 32, "prev hash must be 32 bytes");
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Some(arr)
    };

    let mut rng = Lcg::from_time();
    let mut attempts = 0usize;
    loop {
        attempts += 1;
        assert!(attempts <= 1000, "failed to synthesize compliant series");

        // ---- 随机转向行走（步长 = 合成位移）----
        let disp = synth_displacements(n - 1, ALPHA_TARGET, &mut rng);
        let mut cur = (lat, lng);
        let mut bearing = rng.next_f64() * std::f64::consts::TAU;
        let mut points = vec![cur];
        for &d_km in &disp {
            bearing += (rng.next_f64() - 0.5) * 0.8;
            let d_m = d_km * 1000.0;
            cur.0 += d_m * bearing.cos() / 111_320.0;
            cur.1 += d_m * bearing.sin() / (111_320.0 * cur.0.to_radians().cos());
            points.push(cur);
        }

        // ---- 真实 H3 量化 + 引擎同款预检 ----
        let cells: Vec<u64> = points
            .iter()
            .map(|&(la, lo)| {
                u64::from(
                    h3o::LatLng::new(la, lo)
                        .expect("latlng")
                        .to_cell(h3o::Resolution::try_from(RES).expect("res")),
                )
            })
            .collect();
        let engine_disp = displacements_from_cells(&cells).expect("cells valid");
        let analysis = psd_alpha(&engine_disp).expect("psd");
        let pass = (ALPHA_LO..=ALPHA_HI).contains(&analysis.alpha)
            && analysis.confidence >= MIN_CONF;
        eprintln!(
            "attempt {attempts}: alpha={:.4} r2={:.4} confidence={:.4} {}",
            analysis.alpha,
            analysis.r_squared,
            analysis.confidence,
            if pass { "PASS" } else { "retry" },
        );
        if !pass {
            continue;
        }

        // ---- 预检通过：签名出链（链式 prev_hash）----
        let mut prev = prev_hash;
        let mut crumbs = Vec::with_capacity(n);
        for (k, &(la, lo)) in points.iter().enumerate() {
            let bc = collect_breadcrumb(
                &identity,
                la,
                lo,
                RES,
                start_ts + (k as u64) * gap,
                start_index + k as u64,
                prev,
                false,
                None,
            )
            .expect("collect");
            prev = Some(bc.block_hash());
            crumbs.push(bc);
        }
        let mut file = File::create(&out).expect("create out");
        for bc in &crumbs {
            file.write_all(&bc.to_cbor()).expect("write");
        }
        eprintln!(
            "wrote {} crumbs (index {}..={}, ts {}..={}) to {}",
            crumbs.len(),
            crumbs[0].index,
            crumbs.last().expect("nonempty").index,
            crumbs[0].timestamp,
            crumbs.last().expect("nonempty").timestamp,
            out,
        );
        println!(
            "{} {} {:.4} {:.4}",
            out,
            crumbs.len(),
            analysis.alpha,
            analysis.confidence
        );
        return;
    }
}
