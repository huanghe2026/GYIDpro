//! GyID 生成计时测试
//!
//! 运行：cargo test -p gyid-core --test timing_test -- --nocapture

use gyid_core::{
    GyIdGenerator, GeneratorConfig, GyId, GeoLocation,
    fingerprint::HardwareFingerprint,
    geo::GeoPrecisionLevel,
};
use std::time::Instant;

/// 辅助：格式化毫秒
fn ms(d: std::time::Duration) -> String {
    format!("{:.0} ms", d.as_secs_f64() * 1000.0)
}

// ---- 指纹采集计时（单独拆分） ----
#[tokio::test]
async fn bench_fingerprint_only() {
    println!("\n========== 硬件指纹采集计时 ==========");
    let gen = GyIdGenerator::new(GeneratorConfig {
        with_geo: false,
        with_avatar: false,
        ..Default::default()
    });

    let t = Instant::now();
    let fp: HardwareFingerprint = gen.collect_fingerprint().await.expect("指纹采集失败");
    let elapsed = t.elapsed();

    println!("  有效因子数: {}", fp.valid_factor_count());
    println!("  MAC       : {:?}", fp.mac.as_ref().map(|s| &s[..s.len().min(24)]));
    println!("  CPU ID    : {:?}", fp.cpu_id.as_ref().map(|s| &s[..s.len().min(24)]));
    println!("  板卡序列  : {:?}", fp.board_serial.as_ref().map(|s| &s[..s.len().min(24)]));
    println!("  磁盘序列  : {:?}", fp.disk_serial.as_ref().map(|s| &s[..s.len().min(24)]));
    println!("  ✅ 耗时: {}", ms(elapsed));
    println!("==========================================\n");
}

// ---- City 级（仅 IP 定位）----
#[tokio::test]
async fn bench_generate_city() {
    println!("\n========== GyID 生成计时 · City 级 ==========");
    let gen = GyIdGenerator::new(GeneratorConfig {
        geo_level: GeoPrecisionLevel::City,
        with_geo: true,
        with_avatar: false,
        ..Default::default()
    });

    let t0 = Instant::now();
    let _fp: HardwareFingerprint = gen.collect_fingerprint().await.expect("指纹采集失败");
    let t_fp = t0.elapsed();

    let t1 = Instant::now();
    let geo: GeoLocation = gen.collect_geo().await.expect("地理位置采集失败");
    let t_geo = t1.elapsed();

    let t2 = Instant::now();
    let id: GyId = gen.generate(None).await.expect("生成失败");
    let t_total = t0.elapsed();
    let t_hash = t2.elapsed();

    println!("  地理位置  : {:?}", geo.city);
    println!("  国家      : {:?}", geo.country);
    println!("  坐标      : ({:?}, {:?})", geo.latitude, geo.longitude);
    println!("  GyID      : {}…", &id.id[..id.id.len().min(20)]);
    println!("  ┌─ 指纹采集  : {}", ms(t_fp));
    println!("  ├─ IP 定位   : {}", ms(t_geo));
    println!("  ├─ 哈希/编码 : {}", ms(t_hash));
    println!("  └─ 总计      : {}", ms(t_total));
    println!("===============================================\n");
}

// ---- District 级（WiFi BSSID 定位） ----
#[tokio::test]
async fn bench_generate_district() {
    println!("\n========== GyID 生成计时 · District 级 ==========");
    let gen = GyIdGenerator::new(GeneratorConfig {
        geo_level: GeoPrecisionLevel::District,
        with_geo: true,
        with_avatar: false,
        ..Default::default()
    });

    let t0 = Instant::now();
    let _fp: HardwareFingerprint = gen.collect_fingerprint().await.expect("指纹采集失败");
    let t_fp = t0.elapsed();

    let t1 = Instant::now();
    let geo: GeoLocation = gen.collect_geo().await.expect("地理位置采集失败");
    let t_geo = t1.elapsed();

    let t2 = Instant::now();
    let id: GyId = gen.generate(None).await.expect("生成失败");
    let t_total = t0.elapsed();
    let t_hash = t2.elapsed();

    println!("  地理位置  : {:?}", geo.city);
    println!("  精度等级  : {:?}", geo.level);
    println!("  WiFi BSSID数量: {}", geo.wifi_bssids.as_ref().map_or(0, |v| v.len()));
    println!("  GyID      : {}…", &id.id[..id.id.len().min(20)]);
    println!("  ┌─ 指纹采集  : {}", ms(t_fp));
    println!("  ├─ WiFi定位  : {}", ms(t_geo));
    println!("  ├─ 哈希/编码 : {}", ms(t_hash));
    println!("  └─ 总计      : {}", ms(t_total));
    println!("==================================================\n");
}

// ---- NoGeo 模式（最快路径）----
#[tokio::test]
async fn bench_generate_no_geo() {
    println!("\n========== GyID 生成计时 · NoGeo 模式 ==========");
    let gen = GyIdGenerator::new(GeneratorConfig {
        with_geo: false,
        with_avatar: false,
        ..Default::default()
    });

    let t0 = Instant::now();
    let id: GyId = gen.generate(None).await.expect("生成失败");
    let elapsed = t0.elapsed();

    println!("  GyID   : {}…", &id.id[..id.id.len().min(20)]);
    println!("  ✅ 总计 : {}", ms(elapsed));
    println!("=================================================\n");
}

// ---- 连续 3 次生成，观察 HTTP Client 复用和指纹缓存效果 ----
#[tokio::test]
async fn bench_repeated_city() {
    println!("\n========== 连续 3 次生成 · City 级（HTTP Client + 指纹缓存）==========");
    let gen = GyIdGenerator::new(GeneratorConfig {
        geo_level: GeoPrecisionLevel::City,
        with_geo: true,
        with_avatar: false,
        ..Default::default()
    });

    for i in 1u8..=3 {
        let t = Instant::now();
        let id: GyId = gen.generate(None).await.expect("生成失败");
        let elapsed = t.elapsed();
        println!("  第 {} 次: {}… → {}", i, &id.id[..id.id.len().min(16)], ms(elapsed));
    }
    println!("  💡 注意: 第 2-3 次应该更快（指纹缓存生效）");
    println!("=============================================================\n");
}

// ---- Manual 坐标模式（跳过网络请求）----
#[tokio::test]
async fn bench_generate_manual() {
    println!("\n========== GyID 生成计时 · Manual 坐标模式 ==========");
    let gen = GyIdGenerator::new(GeneratorConfig {
        geo_level: GeoPrecisionLevel::Manual,
        with_geo: true,
        with_avatar: false,
        ..Default::default()
    });

    let test_lat = 39.9042; // 北京
    let test_lon = 116.4074;

    let t0 = Instant::now();
    let id: GyId = gen.generate_with_manual_geo(test_lat, test_lon, None)
        .await.expect("生成失败");
    let elapsed = t0.elapsed();

    println!("  预设坐标  : ({}, {})", test_lat, test_lon);
    println!("  GyID      : {}…", &id.id[..id.id.len().min(20)]);
    println!("  ✅ 总计   : {}", ms(elapsed));
    println!("=====================================================\n");
}
