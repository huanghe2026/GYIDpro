//! PoH 端到端管线示例（W4 验收门）。
//!
//! 单进程模拟完整 TRIP 验证链路：
//! Attester 收集面包屑 → Verifier 构建行为画像 → PSD/Levy/Hamiltonian/trust
//! 评估 → PoH 证书签发 → RP 验签 + 策略检查。
//!
//! 运行：
//!
//! ```bash
//! cargo run --release -p trip-core --example poh_pipeline
//! ```

use rand::rngs::StdRng;
use rand::SeedableRng;
use trip_core::breadcrumb::Breadcrumb;
use trip_core::crypto::ProtocolKey;
use trip_core::engine::behavior::{BehavioralProfile, BreadcrumbView};
use trip_core::engine::hamiltonian::evaluate;
use trip_core::engine::levy::fit as levy_fit;
use trip_core::engine::psd::{displacements_from_cells, psd_alpha};
use trip_core::engine::sim::{trip_walk_path, SimConfig, TripConfig};
use trip_core::engine::trust::{alpha_in_bio_range, can_claim_handle, trust_score, TrustInput};
use trip_core::poh::PohCertificate;

fn main() {
    /* ---------- Attester 侧：生成身份密钥与面包屑链 ---------- */
    let mut rng = StdRng::seed_from_u64(0x4707);
    let identity_key = ProtocolKey::from_seed(&[42u8; 32]);
    let verifier_key = ProtocolKey::from_seed(&[43u8; 32]);

    // 用 trip_walk_path 生成出行结构化 Levy 轨迹（W3 校准的 GeoYuan 参考生成器）。
    // i.i.d. levy_path 产生平坦谱（α≈0，白噪声），不满足桥关系 g∈[0.3,0.7]；
    // trip_walk 的出行结构（Barabási 时间爆发）产生 1/f 记忆，α 中位 ≈ 0.53。
    let cfg = SimConfig {
        raw_step_cap: 20_000,
        ..SimConfig::default()
    };
    let trip = TripConfig::default(); // max_substeps=16, σ=0.50
    let path = trip_walk_path(&mut rng, 1.75, 5.0, 512, &cfg, &trip);
    assert_eq!(path.cells.len(), 513);

    // 构造 Breadcrumb 链（简化：只填关键字段，签名用真实 Ed25519）。
    let id_pub = identity_key.public_bytes();
    let mut crumbs = Vec::with_capacity(513);
    let mut prev_hash: Option<[u8; 32]> = None;
    for (i, &cell) in path.cells.iter().enumerate() {
        let mut bc = Breadcrumb::new_unsigned(
            i as u64,
            id_pub,
            1700000000 + (i as u64) * 900, // 15 分钟间隔
            cell,
            10,        // h3 resolution 10
            [0u8; 32], // context digest（简化）
            prev_hash,
            trip_core::breadcrumb::MetaFlags::new(),
        );
        bc.sign(&identity_key).expect("sign breadcrumb");
        let hash = bc.block_hash();
        crumbs.push((bc, hash));
        prev_hash = Some(hash);
    }

    /* ---------- Verifier 侧：构建画像 + 评估 ---------- */
    let views: Vec<BreadcrumbView> = crumbs
        .iter()
        .map(|(bc, hash)| BreadcrumbView {
            ts: bc.timestamp as i64,
            cell: bc.h3_cell,
            prev_hash: bc.prev_hash,
            block_hash: *hash,
            imu_present: false, // 简化：无 IMU
        })
        .collect();

    let profile = BehavioralProfile::from_breadcrumbs(&views).unwrap();

    // PSD α
    let disp = displacements_from_cells(&path.cells).unwrap();
    let psd = psd_alpha(&disp).unwrap();
    println!(
        "PSD alpha = {:.4} (R² = {:.4}, class = {})",
        psd.alpha,
        psd.r_squared,
        psd.classification.as_str()
    );

    // Levy MLE
    let levy = levy_fit(&disp, None).unwrap();
    println!(
        "Levy beta_hat = {:.4}, kappa_hat = {:.4}",
        levy.beta, levy.kappa
    );

    // Hamiltonian（评估最后一条面包屑）
    let last = views.last().unwrap();
    let prev = &views[views.len() - 2];
    let last_disp = disp.last().copied().unwrap_or(0.0);
    let h_report = evaluate(
        &profile,
        last,
        last_disp,
        &[],               // 无 flock 数据
        Some((0.1, 0.05)), // 简化速度
        prev.ts,
        prev.cell,
        true, // 链完整
    );
    println!(
        "Hamiltonian H = {:.4} (maturity={:.2}, baseline={:.4}, alert={})",
        h_report.total,
        h_report.maturity,
        h_report.baseline,
        h_report.alert.as_str()
    );
    println!(
        "  spatial={:.3} temporal={:.3} kinetic={:.3} flock={:.3} contextual={:.3} structure={:.3}",
        h_report.spatial,
        h_report.temporal,
        h_report.kinetic,
        h_report.flock,
        h_report.contextual,
        h_report.structure
    );

    // 信任分
    let trust_input = TrustInput {
        breadcrumb_count: crumbs.len(),
        unique_cells: profile.unique_cells,
        days_since_first: 1.0, // 简化
        chain_integrity: true,
    };
    let alpha_ok = alpha_in_bio_range(psd.alpha);
    let t = trust_score(&trust_input, alpha_ok);
    println!(
        "Trust T = {:.2} (alpha_in_bio={}, handle_eligible={})",
        t,
        alpha_ok,
        can_claim_handle(&trust_input, alpha_ok)
    );

    /* ---------- PoH 证书签发 ---------- */
    let chain_head = crumbs.last().unwrap().1;
    let nonce = [0xAAu8; 16];
    let cert = PohCertificate::issue(
        &verifier_key,
        identity_key.public_bytes(),
        1700256000,     // issued_at
        2,              // epoch_count
        psd.alpha,      // α
        levy.beta,      // β
        levy.kappa,     // κ
        0.85,           // Π（简化）
        psd.confidence, // criticality confidence
        t,              // trust
        profile.unique_cells as u64,
        crumbs.len() as u64,
        3600, // 1 小时有效
        nonce,
        chain_head,
    );

    // 序列化
    let cbor = cert.to_cbor();
    println!("PoH certificate: {} bytes CBOR", cbor.len());

    /* ---------- RP 侧：验签 + 策略检查 ---------- */
    let parsed = PohCertificate::from_cbor(&cbor).unwrap();
    parsed
        .verify_freshness(&verifier_key.public_bytes(), &nonce, 1700256001)
        .expect("freshness + signature OK");
    // RP 策略：α∈生物区间 + 置信度≥0.1 + T≥20。
    // 置信度公式 = (1 - |α-0.55|/0.25) · R²；512 样本 Levy PSD 的 R² 典型 0.1–0.3，
    // 因此 0.1 是单路径合理的 RP 门槛（生产 RP 可按风险等级调高并要求更多样本）。
    let min_conf = 0.1;
    let policy_ok = parsed.meets_policy(min_conf, 20.0);
    println!(
        "RP verify: signature OK, nonce OK, confidence={:.4}, policy(α∈bio & conf≥{min_conf} & T≥20) = {}",
        parsed.criticality_confidence, policy_ok
    );
    assert!(policy_ok, "RP policy should pass for biological mobility");

    // 篡改检测
    let mut tampered = cbor.clone();
    tampered[20] ^= 1;
    assert!(
        PohCertificate::from_cbor(&tampered).is_err() || {
            let t = PohCertificate::from_cbor(&tampered).unwrap();
            t.verify_freshness(&verifier_key.public_bytes(), &nonce, 1700256001)
                .is_err()
        }
    );
    println!("Tamper detection: OK");

    println!("\n=== W4 PoH pipeline end-to-end: PASS ===");
}
