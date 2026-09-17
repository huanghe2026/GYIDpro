/// 诊断工具: 模拟完整生成流程，不依赖 UI
fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    
    let args: Vec<String> = std::env::args().collect();
    let photo_path = if args.len() > 1 {
        std::path::PathBuf::from(&args[1])
    } else {
        std::path::PathBuf::from(r"C:\Users\Nice\Pictures\GEOYUAN\IMG_20260418_035047.jpg")
    };
    
    println!("=== GeoYuan ID 生成诊断 ===");
    println!("照片: {:?}", photo_path);
    println!();
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_diagnosis(&photo_path).await;
    });
}

async fn run_diagnosis(photo_path: &std::path::Path) {
    use geoyuan_gui::photo::PhotoProcessor;
    use geoyuan_gui::crypto::CryptoService;
    use geoyuan_gui::location::LocationService;

    // 步骤 0
    println!("[0] 检查文件...");
    if !photo_path.exists() {
        eprintln!("  ❌ 文件不存在");
        return;
    }
    println!("  ✅ 文件存在");

    // 步骤 1: GPS 预检
    println!("[1] GPS 预检...");
    match PhotoProcessor::check_gps(photo_path) {
        Ok(Some(true)) => println!("  ✅ GPS 存在"),
        Ok(Some(false)) => { eprintln!("  ❌ 照片无 GPS"); return; }
        Ok(None) => { eprintln!("  ❌ EXIF 读取失败"); return; }
        Err(e) => { eprintln!("  ❌ 文件错误: {}", e); return; }
    }

    // 步骤 2
    println!("[2] 读取 EXIF...");
    let metadata = match PhotoProcessor::extract_metadata(photo_path) {
        Ok(m) => {
            println!("  ✅ EXIF 成功");
            println!("     GPS: {:?}", m.gps.as_ref().map(|g| (g.latitude, g.longitude)));
            println!("     时间戳: {}", m.timestamp);
            m
        }
        Err(e) => { eprintln!("  ❌ EXIF 失败: {}", e); return; }
    };
    
    let gps = match metadata.gps {
        Some(g) => g,
        None => { eprintln!("  ❌ GPS 解析为空"); return; }
    };
    
    // 步骤 3: 高德
    println!("[3] 高德逆地理编码...");
    let service = LocationService::default();
    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        service.verify_and_enhance(gps.clone())
    ).await {
        Ok(Ok(loc)) => println!("  ✅ 地址: {:?}", loc.address),
        Ok(Err(e)) => println!("  ⚠️  高德失败(降级): {}", e),
        Err(_) => println!("  ⚠️  高德超时(降级)"),
    }

    // 步骤 4: 生成 GyID
    println!("[4] 生成 GyID...");
    match CryptoService::generate_gyid(photo_path, gps.latitude, gps.longitude, metadata.timestamp) {
        Ok(id) => {
            println!("  ✅ 生成成功!");
            println!();
            println!("  {}", id);
        }
        Err(e) => eprintln!("  ❌ 生成失败: {}", e),
    }
}
