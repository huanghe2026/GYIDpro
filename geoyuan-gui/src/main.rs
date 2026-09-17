// ═══════════════════════════════════════════════════════════════════════════
// GeoYuan GUI - 主程序入口
// 赛博朋克风格 · Slint 1.8
// 架构: Weak<MainWindow> 可 Send → 子线程中 upgrade() → 直接更新 UI
// ═══════════════════════════════════════════════════════════════════════════

use slint::{ComponentHandle, SharedString};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

slint::include_modules!();

// ═══════════════════════════════════════════════════════════════════════════
// 应用状态
// ═══════════════════════════════════════════════════════════════════════════

mod app;
mod photo;
mod location;
mod crypto;
mod node;
mod achievement;
mod energy;

use app::{AppState, TransactionRecord, TransactionType, TxStatus};
use geoyuan_core::p2p::{Envelope, MessageType, TransferPayload, WalletSyncResponse, TxSummary};
use geoyuan_core::device::DeviceManager;
use photo::PhotoProcessor;
use location::LocationService;
use crypto::CryptoService;
use node::NodeManager;

// 引入 geoyuan-core 的密钥对
use geoyuan_core::crypto::KeyPair;

// ═══════════════════════════════════════════════════════════════════════════
// 主函数
// ═══════════════════════════════════════════════════════════════════════════

fn main() {
    // 设置软件渲染后端（解决 OpenGL 驱动缺失问题）
    std::env::set_var("SLINT_BACKEND", "software");

    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    log::info!("GeoYuan GUI 启动中...");

    let app = MainWindow::new().unwrap();

    // 加载或创建应用状态
    let app_state = Arc::new(Mutex::new({
        let mut state = load_app_state();
        // 每日重置：检查能量恢复 + 重置任务
        let now = chrono::Utc::now().timestamp();
        if state.energy_data.check_daily_reset(now) {
            state.task_manager.reset_daily();
            log::info!("🔋 每日能量已恢复，任务已重置");
        }
        state
    }));

    // 设备管理器
    let device_mgr = Arc::new(Mutex::new({
        let mut mgr = DeviceManager::new();
        {
            let state = app_state.lock().unwrap();
            mgr.import_links(state.linked_devices.clone());
            log::info!("🔗 已恢复 {} 条设备关联记录", state.linked_devices.len());
        }
        mgr
    }));

    // ── 启动 P2P 节点 ────────────────────────────────
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _rt_guard = rt.enter();

    let mut node_mgr = NodeManager::new();
    node_mgr.start();
    let node_status_handle = node_mgr.status_handle();
    // 包装为 Arc<Mutex<>> 以便在 Timer 闭包中调用 sync_status
    let node_mgr_shared = Arc::new(Mutex::new(node_mgr));

    // ── 准备多个 Weak 引用（避免 move 冲突）────────────────
    let app_photo = app.as_weak();
    let app_gen = app.as_weak();
    let _app_copy = app.as_weak();
    let _app_export = app.as_weak();
    let app_wallet = app.as_weak();
    let app_wallet_init = app.as_weak(); // 用于设备信息初始化
    let app_transfer = app.as_weak();

    // 初始化钱包显示
    {
        let state = app_state.lock().unwrap();
        update_wallet_ui(&app_wallet, &state);
        update_history_ui(&app_wallet, &state);
        // 设置转账页可用余额
        if let Some(ui) = app_wallet.upgrade() {
            ui.set_transfer_available(SharedString::from(format!("{:.2}", state.balance)));
        }
        // 推送钱包数据到 Swarm（供其他节点同步查询）
        push_wallet_sync(&state, &node_mgr_shared);
        // 设置同步状态
        if let Some(ui) = app_wallet.upgrade() {
            let sync_status = if state.settings.auto_backup { "已启用" } else { "已关闭" };
            ui.set_wallet_sync_status(SharedString::from(sync_status));
            ui.set_wallet_sync_enabled(state.settings.auto_backup);
            ui.set_amap_key_input(SharedString::from(state.settings.amap_key.clone().unwrap_or_default()));
            ui.set_polygon_rpc_input(SharedString::from(state.settings.polygon_rpc.clone()));
        }
        refresh_explore_ui(&app_wallet, &state);
    }


    // ── 初始化游戏化 UI（成就/能量）───────────────────────────
    {
        let state = app_state.lock().unwrap();
        refresh_game_ui(&app.as_weak(), &state);
        log::info!("🏆 成就系统初始化: {}/{} 已解锁",
            state.achievement_manager.unlocked_count(),
            state.achievement_manager.total_count()
        );
        log::info!("🔋 能量初始化: {:.1}/{}", state.energy_data.current, state.energy_data.max);
    }

    // ── 节点状态定时轮询（每 5 秒刷新 UI + 检查入站转账）────────
    let app_node = app.as_weak();
    let node_status_poll = node_status_handle.clone();
    let node_mgr_timer = node_mgr_shared.clone();
    let app_state_inbound = app_state.clone();
    let app_node_inbound = app.as_weak();
    let _node_timer = slint::Timer::default();
    _node_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(5),
        move || {
            // 先从 SwarmHandle 同步最新状态到缓存
            let (pending_transfers, _wallet_responses) = {
                let mgr = node_mgr_timer.lock().unwrap();
                mgr.sync_status()
            };
            // 再从缓存读取并更新 UI
            let status = node_status_poll.lock().unwrap().clone();
            if let Some(ui) = app_node.upgrade() {
                let net_status = if status.online { "在线" } else { "离线" };
                ui.set_wallet_status(SharedString::from(net_status));
                ui.set_peer_count(status.peer_count as i32);
            }

            // 处理入站转账消息
            if !pending_transfers.is_empty() {
                log::info!("📥 收到 {} 条入站转账通知", pending_transfers.len());
                let mut state = app_state_inbound.lock().unwrap();
                for tx in &pending_transfers {
                    // 只处理发送给本用户的转账（如果 to_gyid 匹配自己的 gy_id）
                    if let Some(ref my_gyid) = state.gy_id {
                        if tx.to_gyid == *my_gyid {
                            let record = TransactionRecord {
                                tx_hash: tx.tx_id.clone(),
                                tx_type: TransactionType::Transfer,
                                amount: tx.amount,
                                from: tx.from_gyid.clone(),
                                to: tx.to_gyid.clone(),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64,
                                status: TxStatus::Confirmed,
                                memo: Some(format!("来自 {} 的转账", &tx.from_gyid[..16.min(tx.from_gyid.len())])),
                            };
                            state.add_transaction(record);
                            state.balance += tx.amount;
                            log::info!("💰 收到转账: +{} GY from {}", tx.amount, &tx.from_gyid[..16.min(tx.from_gyid.len())]);
                        }
                    }
                }
                // 持久化
                if let Err(e) = state.save_to_file(&get_state_path()) {
                    log::error!("❌ 保存入站转账状态失败: {}", e);
                }
                // 更新 UI
                update_wallet_ui(&app_node_inbound, &state);
                update_history_ui(&app_node_inbound, &state);
                // 推送最新钱包数据到 Swarm
                push_wallet_sync(&state, &node_mgr_timer);
            }
        }
    );



    // ── 回调: 请求选择照片 ──────────────────────────────
    app.on_request_photo_pick(move || {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("图片文件", &["jpg", "jpeg", "png", "heic", "webp"])
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            log::info!("📷 照片已选择: {}", path_str);
            if let Some(ui) = app_photo.upgrade() {
                ui.set_photo_path(SharedString::from(path_str.as_str()));
            }
        }
    });

    // ── 回调: 开始生成 ────────────────────────────────
    let state_gen_arc = app_state.clone();
    app.on_start_generate(move |path| {
        let path_str = path.to_string();
        log::info!("🚀 开始生成 GyID: {}", path_str);

        // 立即更新 UI（同步，在 Slint 事件循环中直接执行）
        if let Some(ui) = app_gen.upgrade() {
            ui.set_error_text(SharedString::from(""));
            ui.set_location_text(SharedString::from("准备中..."));
            ui.set_progress(0.02);
        }

        let path_owned = PathBuf::from(path_str);
        let app_bg = app_gen.clone();
        let state_gen = state_gen_arc.clone();

        std::thread::spawn(move || {
            // 在独立线程中创建 tokio runtime 并运行异步流程
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(async {
                // 给 Slint 一点时间完成页面切换
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                run_registration(path_owned, app_bg.clone()).await
            });

            // 必须通过 invoke_from_event_loop 回到 UI 线程更新界面
            let app_final = app_bg.clone();
            slint::invoke_from_event_loop(move || {
                if let Some(ui) = app_final.upgrade() {
                    match result {
                        Ok(reg) => {
                            let gy_id = reg.gy_id;
                            ui.set_gy_id(SharedString::from(gy_id.as_str()));
                            ui.set_progress(1.0);
                            ui.set_location_text(SharedString::from("铸造完成!"));
                            ui.set_current_page(SharedString::from("success"));
                            log::info!("✅ GyID 生成成功: {}", gy_id);

                            // 成就检查: 首次铸造 + 时间成就
                            let now_ts = chrono::Utc::now().timestamp();
                            if let Ok(mut state) = state_gen.lock() {
                                let balance_f32 = state.balance as f32;
                                let unlocked = state.achievement_manager.check_achievements(
                                    &reg.city, balance_f32, now_ts
                                );
                                if !unlocked.is_empty() {
                                    log::info!("🏆 新成就解锁: {:?}", unlocked);
                                }
                                // 每日任务: 铸造完成
                                if let Some(reward) = state.task_manager.complete_task("daily_mint") {
                                    state.energy_data.add(reward);
                                    log::info!("🔋 完成每日铸造任务，奖励 {} 能量", reward);
                                }
                                // 更新余额（首次铸造奖励）
                                state.balance += 1.0;
                                // 记录铸造位置
                                state.mint_locations.push(app::MintLocation {
                                    gyid: gy_id.clone(),
                                    city: reg.city.clone(),
                                    address: reg.address.clone(),
                                    lat: reg.lat,
                                    lon: reg.lon,
                                    timestamp: reg.timestamp,
                                });
                                // 保存
                                if let Err(e) = state.save_to_file(&get_state_path()) {
                                    log::error!("❌ 保存成就状态失败: {}", e);
                                }
                                refresh_game_ui(&app_final, &state);
                                refresh_explore_ui(&app_final, &state);
                            }
                        }
                        Err(e) => {
                            let err_str = e.to_string();
                            log::error!("❌ 生成失败: {}", err_str);
                            ui.set_error_text(SharedString::from(err_str.as_str()));
                            ui.set_location_text(SharedString::from("⚠️ 生成失败，请返回重试"));
                            ui.set_progress(0.0);
                            // 留在 locating 页显示错误信息（该页有 error_text 显示区域）
                        }
                    }
                }
            }).ok();
        });
    });

    // ── 回调: 复制 GyID ────────────────────────────
    app.on_copy_gyid(move |gy_id| {
        let gy_id = gy_id.to_string();
        let mut clipboard = match arboard::Clipboard::new() {
            Ok(c) => c,
            Err(e) => { log::error!("无法访问剪贴板: {}", e); return; }
        };
        if let Err(e) = clipboard.set_text(gy_id) {
            log::error!("复制失败: {}", e);
        } else {
            log::info!("📋 GyID 已复制到剪贴板");
        }
    });

    // ── 回调: 导出 JSON ─────────────────────────────
    app.on_export_json(move |gy_id| {
        let gy_id = gy_id.to_string();
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("geoyuan_identity.json")
            .add_filter("JSON", &["json"])
            .save_file()
        {
            let json = serde_json::json!({
                "gy_id": gy_id,
                "exported_at": chrono::Utc::now().to_rfc3339(),
                "version": "1.0"
            });
            if let Err(e) = std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap()) {
                log::error!("导出失败: {}", e);
            } else {
                log::info!("📄 JSON 已导出到: {:?}", path);
            }
        }
    });

    // ── 回调: 验证 GyID ─────────────────────────────
    let app_verify = app.as_weak();
    app.on_verify_gyid(move |input| {
        let input = input.to_string();
        log::info!("🔐 验证 GyID: {}", input);

        if let Some(ui) = app_verify.upgrade() {
            match CryptoService::verify_gyid(&input) {
                Ok(valid) => {
                    if valid {
                        // 提取哈希用于展示
                        let hash_result = CryptoService::extract_hash(&input);
                        let hash_str = match hash_result {
                            Ok(h) => hex::encode(&h[..16]),
                            Err(_) => "N/A".to_string(),
                        };
                        ui.set_verify_valid(true);
                        ui.set_verify_result(SharedString::from("GyID 格式正确，校验和验证通过"));
                        ui.set_verify_hash(SharedString::from(&hash_str));
                        log::info!("✅ GyID 验证通过: {}", hash_str);
                    } else {
                        ui.set_verify_valid(false);
                        ui.set_verify_result(SharedString::from("校验和验证失败"));
                        ui.set_verify_hash(SharedString::from(""));
                        log::warn!("❌ GyID 校验和验证失败");
                    }
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    ui.set_verify_valid(false);
                    ui.set_verify_result(SharedString::from(&err_msg));
                    ui.set_verify_hash(SharedString::from(""));
                    log::error!("❌ GyID 验证错误: {}", err_msg);
                }
            }
        }
    });

    // ── 回调: 从剪贴板粘贴 GyID ─────────────────────────
    let app_paste = app.as_weak();
    app.on_paste_gyid(move || {
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => match clipboard.get_text() {
                Ok(text) => {
                    let trimmed = text.trim().to_string();
                    log::info!("📋 剪贴板内容: {}", trimmed);
                    if let Some(ui) = app_paste.upgrade() {
                        ui.set_verify_input(SharedString::from(&trimmed));
                        if trimmed.starts_with("GyID") || trimmed.starts_with("TGyID") {
                            ui.set_verify_result(SharedString::from("已粘贴，请点击验证"));
                        } else {
                            ui.set_verify_result(SharedString::from("剪贴板内容不是有效的 GyID"));
                        }
                    }
                }
                Err(e) => {
                    log::warn!("⚠️ 剪贴板读取失败: {}", e);
                    if let Some(ui) = app_paste.upgrade() {
                        ui.set_verify_result(SharedString::from("剪贴板读取失败"));
                    }
                }
            },
            Err(e) => {
                log::error!("❌ 无法初始化剪贴板: {}", e);
                if let Some(ui) = app_paste.upgrade() {
                    ui.set_verify_result(SharedString::from("无法访问剪贴板"));
                }
            }
        }
    });

    // ── 回调: 扫描二维码 ───────────────────────────
    let app_scan = app.as_weak();
    app.on_scan_qr_code(move || {
        log::info!("📷 打开二维码扫描器...");

        // 从相册选择图片
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("图片文件", &["jpg", "jpeg", "png"])
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            log::info!("📷 扫描图片: {}", path_str);

            // 使用 rqrr 解码二维码
            match image::open(&path_str) {
                Ok(img) => {
                    let gray = img.to_luma8();
                    let mut prepared = rqrr::PreparedImage::prepare(gray);
                    let grids = prepared.detect_grids();

                    match grids.first() {
                        Some(grid) => match grid.decode() {
                            Ok((_, content)) => {
                                let data_str = content;
                                log::info!("📷 二维码内容: {}", data_str);

                                if let Some(ui) = app_scan.upgrade() {
                                    if data_str.starts_with("GyID") || data_str.starts_with("TGyID") {
                                        ui.set_verify_input(SharedString::from(&data_str));
                                        ui.set_verify_result(SharedString::from("二维码扫描成功，请点击验证"));
                                    } else {
                                        ui.set_verify_input(SharedString::from(&data_str));
                                        ui.set_verify_result(SharedString::from("二维码内容不是有效的 GyID"));
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("❌ 二维码解码失败: {:?}", e);
                                if let Some(ui) = app_scan.upgrade() {
                                    ui.set_verify_result(SharedString::from("二维码解码失败，请确保图片清晰"));
                                }
                            }
                        },
                        None => {
                            log::warn!("⚠️ 未检测到二维码");
                            if let Some(ui) = app_scan.upgrade() {
                                ui.set_verify_result(SharedString::from("未检测到二维码，请确保图片包含二维码"));
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("❌ 图片读取失败: {}", e);
                    if let Some(ui) = app_scan.upgrade() {
                        ui.set_verify_result(SharedString::from(&format!("图片读取失败: {}", e)));
                    }
                }
            }
        }
    });

    // ── 回调: 刷新钱包 ─────────────────────────────
    let app_state_wallet = app_state.clone();
    let app_state_history = app_state.clone();
    app.on_refresh_wallet(move || {
        let state = app_state_wallet.lock().unwrap();
        update_wallet_ui(&app_wallet, &state);
        update_history_ui(&app_wallet, &state);
        log::info!("💰 钱包数据已刷新");
    });

    // ── 回调: 加载交易历史 ─────────────────────────────
    let app_history = app.as_weak();
    app.on_load_transactions(move || {
        let state = app_state_history.lock().unwrap();
        update_history_ui(&app_history, &state);
        log::info!("📜 交易历史已加载");
    });

    // ── 回调: 发送转账 ─────────────────────────────
    let app_state_tx = app_state.clone();
    let node_mgr_broadcast = node_mgr_shared.clone();
    app.on_send_transfer(move |to_address, amount| {
        let to = to_address.to_string();
        let amt_str = amount.to_string();
        log::info!("💸 发起转账: {} GY → {}", amt_str, to);

        // ① 验证地址格式（GyID 前缀 或 TGyID 前缀）
        if !to.starts_with("GyID") && !to.starts_with("TGyID") && !to.starts_with("0x") {
            if let Some(ui) = app_transfer.upgrade() {
                ui.set_transfer_msg(SharedString::from(
                    "⚠️ 无效地址：请输入 GyID/TGyID 或 0x 开头的地址"
                ));
            }
            return;
        }

        // ② 解析金额
        let amt: f64 = match amt_str.parse() {
            Ok(v) if v > 0.0 => v,
            _ => {
                if let Some(ui) = app_transfer.upgrade() {
                    ui.set_transfer_msg(SharedString::from("⚠️ 无效金额"));
                }
                return;
            }
        };

        // ③ 检查余额
        let current_balance = {
            let state = app_state_tx.lock().unwrap();
            state.balance
        };

        if amt > current_balance {
            if let Some(ui) = app_transfer.upgrade() {
                ui.set_transfer_msg(SharedString::from(
                    format!("⚠️ 余额不足 (持有 {:.2} GY)", current_balance).as_str()
                ));
            }
            return;
        }

        // ④ 执行转账 — 使用 Ed25519 签名生成转账凭证
        let transfer_result = {
            let mut state = app_state_tx.lock().unwrap();

            // 获取或生成密钥对（正确存储私钥种子而非公钥）
            let keypair = if let Some(ref hex) = state.secret_key_hex {
                let bytes = hex::decode(hex).unwrap_or_default();
                if bytes.len() >= 32 {
                    let arr: [u8; 32] = bytes[..32].try_into().unwrap_or([0u8; 32]);
                    KeyPair::from_secret(&arr).unwrap_or_else(|_| {
                        let kp = KeyPair::generate();
                        state.secret_key_hex = Some(hex::encode(kp.secret_bytes()));
                        kp
                    })
                } else {
                    // 存储的是旧的公钥格式（64 hex = 32 bytes），无法恢复私钥，重新生成
                    log::warn!("⚠️ 密钥格式不兼容，重新生成密钥对");
                    let kp = KeyPair::generate();
                    state.secret_key_hex = Some(hex::encode(kp.secret_bytes()));
                    kp
                }
            } else {
                let kp = KeyPair::generate();
                state.secret_key_hex = Some(hex::encode(kp.secret_bytes()));
                kp
            };

            // 获取发送方 GyID
            let from_gyid = state.gy_id.clone().unwrap_or_else(|| "unknown".to_string());
            let pub_hex = keypair.public_hex();

            // 使用 Ed25519 签名消息：to + amount + timestamp
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let msg = format!("{}:{}:{}", to, amt_str, timestamp);
            let signature = keypair.sign(msg.as_bytes());
            let sig_hex = hex::encode(signature.to_bytes());
            let tx_id = format!("0x{}", &sig_hex[..32]);

            // 扣减余额
            state.balance -= amt;

            // 写入交易记录
            let tx_record = TransactionRecord {
                tx_hash: tx_id.clone(),
                tx_type: TransactionType::Transfer,
                amount: amt,
                from: from_gyid.clone(),
                to: to.clone(),
                timestamp,
                status: TxStatus::Confirmed,
                memo: None,
            };
            state.add_transaction(tx_record);

            // 持久化状态（含交易记录和密钥）
            if let Err(e) = state.save_to_file(&get_state_path()) {
                log::error!("❌ 保存状态失败: {}", e);
            } else {
                log::info!("💾 状态已持久化 (含交易记录)");
            }

            (tx_id, sig_hex, pub_hex, from_gyid)
        };

        let (tx_id, _sig_hex, pub_hex, from_gyid) = transfer_result;
        log::info!(
            "✅ 转账已签名: tx={}, pubkey={}..., to={}, amount={} GY",
            &tx_id[..20.min(tx_id.len())], &pub_hex[..16.min(pub_hex.len())], to, amt
        );

        // ⑤ 通过 P2P 网络广播 CoinTransferred 消息
        {

            let payload = TransferPayload {
                tx_id: tx_id.clone(),
                from_gyid: from_gyid.clone(),
                to_gyid: to.clone(),
                amount: amt,
                sender_pubkey: pub_hex.clone(),
            };

            let payload_bytes = match serde_json::to_vec(&payload) {
                Ok(b) => b,
                Err(e) => {
                    log::error!("❌ 序列化 TransferPayload 失败: {}", e);
                    if let Some(ui) = app_transfer.upgrade() {
                        ui.set_transfer_msg(SharedString::from(
                            format!("✅ 转账已签名\nhash: {}\n⚠️ P2P广播序列化失败", &tx_id[..20.min(tx_id.len())]).as_str()
                        ));
                    }
                    return;
                }
            };

            // 构造签名 Envelope 并通过 CBOR 序列化
            let envelope = Envelope::new_signed(
                MessageType::CoinTransferred,
                &from_gyid,
                payload_bytes,
                |_data| {
                    // Envelope 签名使用交易签名的前 64 字节
                    // (更安全的做法是用独立签名，此处复用 tx 签名)
                    tx_id[2..].to_string()
                },
            );

            let envelope_bytes = match envelope.serialize_cbor() {
                Ok(b) => b,
                Err(e) => {
                    log::error!("❌ 序列化 Envelope CBOR 失败: {}", e);
                    if let Some(ui) = app_transfer.upgrade() {
                        ui.set_transfer_msg(SharedString::from(
                            format!("✅ 转账已签名\nhash: {}\n⚠️ P2P广播失败", &tx_id[..20.min(tx_id.len())]).as_str()
                        ));
                    }
                    return;
                }
            };

            let mgr = node_mgr_broadcast.lock().unwrap();
            mgr.broadcast(MessageType::CoinTransferred, envelope_bytes);
        }

        // ⑥ 更新 UI
        if let Some(ui) = app_transfer.upgrade() {
            ui.set_transfer_msg(SharedString::from(
                format!("✅ 转账已广播\nhash: {}", &tx_id[..20.min(tx_id.len())]).as_str()
            ));
            // 刷新余额显示
            let mut state = app_state_tx.lock().unwrap();
            let new_balance = format!("{:.2}", state.balance);
            ui.set_gy_balance(SharedString::from(&new_balance));
            ui.set_transfer_available(SharedString::from(&new_balance));
            // 刷新历史记录
            update_history_ui(&app_transfer, &state);
            // 推送最新钱包数据到 Swarm
            push_wallet_sync(&state, &node_mgr_broadcast);

            // 成就检查: 富翁（持有 100 GY）
            let now_ts = chrono::Utc::now().timestamp();
            let balance_f32 = state.balance as f32;
            let unlocked = state.achievement_manager.check_achievements("本地", balance_f32, now_ts);
            if !unlocked.is_empty() {
                log::info!("🏆 新成就解锁（转账后）: {:?}", unlocked);
            }
            refresh_game_ui(&app_transfer, &state);
        }
    });

    // ── 回调: 生成授权码（主设备模式）─────────────────────
    let app_gen_code = app.as_weak();
    let device_mgr_gen = device_mgr.clone();
    let app_state_gen = app_state.clone();
    app.on_generate_auth_code(move || {
        let state = app_state_gen.lock().unwrap();
        let master_gyid = match &state.gy_id {
            Some(id) => id.clone(),
            None => {
                if let Some(ui) = app_gen_code.upgrade() {
                    ui.set_device_link_result(SharedString::from("⚠️ 请先生成 GyID"));
                }
                return;
            }
        };
        drop(state);

        let code;
        {
            let mut mgr = device_mgr_gen.lock().unwrap();
            code = mgr.generate_auth_code(&master_gyid);
            mgr.save_pending_auth_code(&code, &master_gyid);
        }

        let result_msg = format!("✅ 授权码已生成: {}\n\n在新设备上点击\"关联设备\"并输入此授权码。\n有效期 10 分钟。", code);
        log::info!("🔑 授权码生成: {}", code);

        if let Some(ui) = app_gen_code.upgrade() {
            ui.set_device_link_result(SharedString::from(result_msg));
            ui.set_device_link_role(SharedString::from("主设备"));
            ui.set_device_link_master(SharedString::from(&master_gyid));
        }
    });

    // ── 回调: 使用授权码关联设备（从设备模式）────────────────
    let app_link_dev = app.as_weak();
    let device_mgr_link = device_mgr.clone();
    let app_state_link = app_state.clone();
    app.on_link_device(move |auth_code| {
        let code_str = auth_code.to_string();
        if code_str.trim().is_empty() {
            if let Some(ui) = app_link_dev.upgrade() {
                ui.set_device_link_result(SharedString::from("⚠️ 请输入授权码"));
            }
            return;
        }

        let state = app_state_link.lock().unwrap();
        let local_gyid = match &state.gy_id {
            Some(id) => id.clone(),
            None => {
                if let Some(ui) = app_link_dev.upgrade() {
                    ui.set_device_link_result(SharedString::from("⚠️ 请先生成 GyID"));
                }
                return;
            }
        };
        drop(state);

        let master_gyid;
        let result;
        {
            let mut mgr = device_mgr_link.lock().unwrap();

            // 尝试从 pending auth codes 获取 master_gyid
            if let Some((m, _)) = mgr.pending_auth_codes.get(&code_str) {
                master_gyid = m.clone();
            } else {
                // 没有缓存，需要用户提供，暂用本地 GyID 作为提示
                master_gyid = String::new();
            }

            if master_gyid.is_empty() {
                result = "⚠️ 未找到主设备信息，请先在本机生成授权码。".to_string();
            } else if !mgr.verify_auth_code(&code_str, &master_gyid) {
                result = "❌ 授权码无效或已过期（10 分钟有效）。".to_string();
            } else {
            match mgr.create_link(&master_gyid, &local_gyid, &code_str) {
                Ok(link) => {
                    let count = mgr.active_link_count(&master_gyid);
                    result = format!(
                        "✅ 设备关联成功！\n  主设备: {}\n  本机 GyID: {}\n  关联 ID: {}\n  当前关联设备: {} 台",
                        &master_gyid[..16.min(master_gyid.len())],
                        &local_gyid[..16.min(local_gyid.len())],
                        &link.link_id[..16.min(link.link_id.len())],
                        count
                    );
                    log::info!("🔗 设备关联成功: {} → {}", &master_gyid[..12.min(master_gyid.len())], &local_gyid[..12.min(local_gyid.len())]);
                    // 持久化到 AppState
                    let mut state = app_state_link.lock().unwrap();
                    state.linked_devices.push(link);
                    if let Err(e) = state.save_to_file(&get_state_path()) {
                        log::error!("❌ 保存设备关联失败: {}", e);
                    }
                }
                Err(e) => {
                    result = format!("❌ 关联失败: {}", e);
                    log::warn!("⚠️ 设备关联失败: {}", e);
                }
            }
            }
        }

        let link_count;
        {
            let mgr = device_mgr_link.lock().unwrap();
            link_count = if master_gyid.is_empty() { 0 } else { mgr.active_link_count(&master_gyid) };
        }

        if let Some(ui) = app_link_dev.upgrade() {
            ui.set_device_link_result(SharedString::from(result));
            ui.set_device_link_count(link_count as i32);
            if !master_gyid.is_empty() {
                ui.set_device_link_role(SharedString::from("从设备"));
                ui.set_device_link_master(SharedString::from(&master_gyid));
            }
        }
    });

    // ── 回调: 加载设备列表 ────────────────────────────
    let app_load_dev = app.as_weak();
    let app_state_load = app_state.clone();
    app.on_load_device_list(move || {
        let state = app_state_load.lock().unwrap();
        let master_gyid = state.gy_id.clone().unwrap_or_default();

        let owned_links: Vec<&geoyuan_core::device::DeviceLink> = state
            .linked_devices
            .iter()
            .filter(|l| l.master_gyid == master_gyid && l.active)
            .collect();
        let count = owned_links.len();
        let links: Vec<String> = owned_links.iter().map(|l| {
            let gyid_short = if l.linked_gyid.len() > 12 {
                format!("{}...", &l.linked_gyid[..12])
            } else {
                l.linked_gyid.clone()
            };
            let time = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(l.linked_at as i64)
                .map(|t| t.format("%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "未知".to_string());
            let status = if l.active { "🟢 活跃" } else { "🔴 停用" };
            format!("  {}. {}  {}  {}", gyid_short, time, status, &l.link_id[..12.min(l.link_id.len())])
        }).collect();
        drop(state);

        let list_text = if links.is_empty() {
            "暂无关联设备。\n使用授权码功能添加新设备。".to_string()
        } else {
            links.join("\n")
        };

        if let Some(ui) = app_load_dev.upgrade() {
            ui.set_device_list_text(SharedString::from(list_text));
            ui.set_device_link_count(count as i32);
        }
    });

    // 回调：保存设置
    let app_save_settings = app.as_weak();
    let app_state_save_settings = app_state.clone();
    app.on_save_settings(move |amap_key, polygon_rpc, wallet_sync| {
        let mut state = app_state_save_settings.lock().unwrap();
        state.settings.amap_key = if amap_key.is_empty() { None } else { Some(amap_key.to_string()) };
        state.settings.polygon_rpc = polygon_rpc.to_string();
        state.settings.auto_backup = wallet_sync;
        if let Err(e) = state.save_to_file(&get_state_path()) {
            log::error!("❌ 保存设置失败: {}", e);
            if let Some(ui) = app_save_settings.upgrade() {
                ui.set_settings_save_result(SharedString::from("保存失败"));
            }
        } else {
            log::info!("✅ 设置已保存");
            if let Some(ui) = app_save_settings.upgrade() {
                ui.set_settings_save_result(SharedString::from("保存成功"));
                ui.set_wallet_sync_status(SharedString::from(if wallet_sync { "已启用" } else { "已关闭" }));
                ui.set_wallet_sync_enabled(wallet_sync);
            }
        }
    });

    // 初始化设备信息到 UI
    {
        let state = app_state.lock().unwrap();
        let mgr = device_mgr.lock().unwrap();
        if let Some(ref gyid) = state.gy_id {
            let count = mgr.active_link_count(gyid);
            if let Some(ui) = app_wallet_init.upgrade() {
                ui.set_device_link_master(SharedString::from(gyid));
                ui.set_device_link_role(SharedString::from("主设备"));
                ui.set_device_link_count(count as i32);
            }
        }
    }

    // 运行 UI
    app.run().unwrap();

    // 保存应用状态
    save_app_state(&app_state.lock().unwrap());

    // 停止 P2P 节点（node_mgr_shared 在此 Drop）
    drop(node_mgr_shared);
}

// ═══════════════════════════════════════════════════════════════════════════
// UI 更新辅助函数（线程安全）
// ═══════════════════════════════════════════════════════════════════════════

/// 刷新探索页 UI
fn refresh_explore_ui(app_weak: &slint::Weak<MainWindow>, state: &AppState) {
    let total_gyids = state.mint_locations.len() as i32;
    let total_locations: usize = state
        .mint_locations
        .iter()
        .map(|l| l.city.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let coverage = if total_locations > 0 {
        ((total_locations as f32 / 195.0) * 100.0).min(100.0)
    } else {
        0.0
    };
    let coverage_text = format!("{:.0}%", coverage);

    let app_weak = app_weak.clone();
    slint::invoke_from_event_loop(move || {
        if let Some(ui) = app_weak.upgrade() {
            ui.set_total_gyids(total_gyids);
            ui.set_total_locations(total_locations as i32);
            ui.set_coverage_text(SharedString::from(coverage_text.as_str()));
        }
    }).ok();
}

/// 刷新游戏化 UI（成就 + 能量）
fn refresh_game_ui(app_weak: &slint::Weak<MainWindow>, state: &AppState) {
    let unlocked = state.achievement_manager.unlocked_count() as i32;
    let total = state.achievement_manager.total_count() as i32;
    let energy_current = state.energy_data.current;
    let energy_max = state.energy_data.max;

    let app_weak = app_weak.clone();
    slint::invoke_from_event_loop(move || {
        if let Some(ui) = app_weak.upgrade() {
            ui.set_achievements_unlocked(unlocked);
            ui.set_achievements_total(total);
            ui.set_energy(energy_current);
            ui.set_energy_max(energy_max);
        }
    }).ok();
}

fn update_ui_location(app_weak: &slint::Weak<MainWindow>, text: &str) {
    let text = text.to_string();
    let app_weak = app_weak.clone();
    let result = slint::invoke_from_event_loop(move || {
        if let Some(ui) = app_weak.upgrade() {
            ui.set_location_text(SharedString::from(text.as_str()));
        } else {
            log::warn!("⚠️ UI upgrade 失败 — 无法更新 location_text");
        }
    });
    if let Err(e) = result {
        log::error!("❌ invoke_from_event_loop 失败: {:?}", e);
    }
}

fn update_ui_progress(app_weak: &slint::Weak<MainWindow>, progress: f32) {
    let app_weak = app_weak.clone();
    slint::invoke_from_event_loop(move || {
        if let Some(ui) = app_weak.upgrade() {
            ui.set_progress(progress);
        }
    }).ok();
}

fn update_ui_error(app_weak: &slint::Weak<MainWindow>, text: &str) {
    let text = text.to_string();
    let app_weak = app_weak.clone();
    slint::invoke_from_event_loop(move || {
        if let Some(ui) = app_weak.upgrade() {
            ui.set_error_text(SharedString::from(text.as_str()));
        }
    }).ok();
}

// ═══════════════════════════════════════════════════════════════════════════
// 钱包 UI 更新
// ═══════════════════════════════════════════════════════════════════════════

fn update_wallet_ui(app_weak: &slint::Weak<MainWindow>, state: &AppState) {
    let balance = format!("{:.2}", state.balance);
    let address = state.wallet_address.clone().unwrap_or_else(|| "0x0000...0000".to_string());
    let status = if state.node_status.online { "在线" } else { "离线" };
    let peers = state.node_status.peer_count as i32;

    let app_weak = app_weak.clone();
    slint::invoke_from_event_loop(move || {
        if let Some(ui) = app_weak.upgrade() {
            ui.set_gy_balance(SharedString::from(&balance));
            ui.set_wallet_address(SharedString::from(&address));
            ui.set_wallet_status(SharedString::from(status));
            ui.set_peer_count(peers);
            ui.set_transfer_available(SharedString::from(&balance));
        }
    }).ok();
}

/// 构建 WalletSyncResponse 并推送到 SwarmHandle
/// 在每次余额/交易变化后调用，使其他节点能查到最新数据
fn push_wallet_sync(state: &AppState, node_mgr: &Arc<Mutex<NodeManager>>) {
    let gyid = state.gy_id.clone().unwrap_or_default();
    let tx_summaries: Vec<TxSummary> = state.transactions.iter().take(10).map(|tx| {
        TxSummary {
            tx_hash_short: tx.tx_hash[..20.min(tx.tx_hash.len())].to_string(),
            tx_type: tx.tx_type.to_string(),
            amount: tx.amount,
            timestamp: tx.timestamp,
        }
    }).collect();

    let response = WalletSyncResponse {
        responder_gyid: gyid,
        balance: state.balance,
        tx_count: state.transactions.len(),
        recent_txs: tx_summaries,
        timestamp: chrono::Utc::now().timestamp(),
    };

    if let Ok(mgr) = node_mgr.lock() {
        mgr.set_wallet_data(response);
    }
}

// ═══════════════════════════════════════════════════════════════
// 交易历史 UI 更新
// ═══════════════════════════════════════════════════════════════

/// 将交易记录格式化为 Slint UI 可显示的属性
fn update_history_ui(app_weak: &slint::Weak<MainWindow>, state: &AppState) {
    let txs = &state.transactions;
    let total = txs.len();
    let mint_count = txs.iter().filter(|t| t.tx_type == TransactionType::Mint).count();
    let transfer_count = txs.iter().filter(|t| t.tx_type == TransactionType::Transfer).count();

    // 取前 5 条交易，转换为显示字符串
    let display_txs: Vec<(String, String, String, String, String)> = txs.iter().take(5).map(|tx| {
        let icon = match tx.tx_type {
            TransactionType::Mint => "🪙",
            TransactionType::Transfer => "💸",
            TransactionType::Link => "🔗",
            TransactionType::Reward => "🎁",
        };
        let title = match tx.tx_type {
            TransactionType::Mint => "铸造 GyID".to_string(),
            TransactionType::Transfer => format!("转账 → {}", &tx.to[..12.min(tx.to.len())]),
            TransactionType::Link => "设备关联".to_string(),
            TransactionType::Reward => "奖励".to_string(),
        };
        let amount = format!("{:+.2} GY", tx.amount);
        let status = tx.status.to_string();
        let time = format_timestamp(tx.timestamp);
        (icon.into(), title, amount, time, status)
    }).collect();

    let app_weak = app_weak.clone();
    slint::invoke_from_event_loop(move || {
        if let Some(ui) = app_weak.upgrade() {
            // 历史页统计
            ui.set_history_count(SharedString::from(total.to_string()));
            ui.set_history_mint_count(SharedString::from(mint_count.to_string()));
            ui.set_history_transfer_count(SharedString::from(transfer_count.to_string()));

            // 历史页交易列表（最多 5 条）
            for (i, (icon, title, amount, time, status)) in display_txs.iter().enumerate() {
                match i {
                    0 => { ui.set_tx1_icon(SharedString::from(icon)); ui.set_tx1_title(SharedString::from(title)); ui.set_tx1_amount(SharedString::from(amount)); ui.set_tx1_time(SharedString::from(time)); ui.set_tx1_status(SharedString::from(status)); }
                    1 => { ui.set_tx2_icon(SharedString::from(icon)); ui.set_tx2_title(SharedString::from(title)); ui.set_tx2_amount(SharedString::from(amount)); ui.set_tx2_time(SharedString::from(time)); ui.set_tx2_status(SharedString::from(status)); }
                    2 => { ui.set_tx3_icon(SharedString::from(icon)); ui.set_tx3_title(SharedString::from(title)); ui.set_tx3_amount(SharedString::from(amount)); ui.set_tx3_time(SharedString::from(time)); ui.set_tx3_status(SharedString::from(status)); }
                    3 => { ui.set_tx4_icon(SharedString::from(icon)); ui.set_tx4_title(SharedString::from(title)); ui.set_tx4_amount(SharedString::from(amount)); ui.set_tx4_time(SharedString::from(time)); ui.set_tx4_status(SharedString::from(status)); }
                    4 => { ui.set_tx5_icon(SharedString::from(icon)); ui.set_tx5_title(SharedString::from(title)); ui.set_tx5_amount(SharedString::from(amount)); ui.set_tx5_time(SharedString::from(time)); ui.set_tx5_status(SharedString::from(status)); }
                    _ => {}
                }
            }
            // 清空多余项
            for i in display_txs.len()..5 {
                match i {
                    0 => { ui.set_tx1_title(SharedString::from("")); }
                    1 => { ui.set_tx2_title(SharedString::from("")); }
                    2 => { ui.set_tx3_title(SharedString::from("")); }
                    3 => { ui.set_tx4_title(SharedString::from("")); }
                    4 => { ui.set_tx5_title(SharedString::from("")); }
                    _ => {}
                }
            }

            // 钱包页最近交易（前 2 条）
            for (i, (icon, title, amount, time, status)) in display_txs.iter().take(2).enumerate() {
                match i {
                    0 => { ui.set_wallet_tx1_icon(SharedString::from(icon)); ui.set_wallet_tx1_title(SharedString::from(title)); ui.set_wallet_tx1_amount(SharedString::from(amount)); ui.set_wallet_tx1_time(SharedString::from(time)); ui.set_wallet_tx1_status(SharedString::from(status)); }
                    1 => { ui.set_wallet_tx2_icon(SharedString::from(icon)); ui.set_wallet_tx2_title(SharedString::from(title)); ui.set_wallet_tx2_amount(SharedString::from(amount)); ui.set_wallet_tx2_time(SharedString::from(time)); ui.set_wallet_tx2_status(SharedString::from(status)); }
                    _ => {}
                }
            }
            // 清空多余项
            for i in display_txs.len().min(2)..2 {
                match i {
                    0 => { ui.set_wallet_tx1_title(SharedString::from("")); }
                    1 => { ui.set_wallet_tx2_title(SharedString::from("")); }
                    _ => {}
                }
            }
        }
    }).ok();
}

/// 格式化时间戳为可读字符串
fn format_timestamp(ts: u64) -> String {
    use std::time::UNIX_EPOCH;
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let diff_ms = now.saturating_sub(ts);
    let diff_secs = diff_ms / 1000;

    if diff_secs < 60 {
        "刚刚".to_string()
    } else if diff_secs < 3600 {
        format!("{} 分钟前", diff_secs / 60)
    } else if diff_secs < 86400 {
        format!("{} 小时前", diff_secs / 3600)
    } else {
        format!("{} 天前", diff_secs / 86400)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 应用状态持久化
// ═══════════════════════════════════════════════════════════════════════════

fn get_state_path() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("geoyuan");
    path.push("app_state.json");
    path
}

fn load_app_state() -> AppState {
    let path = get_state_path();
    if path.exists() {
        match AppState::load_from_file(&path) {
            Ok(state) => {
                log::info!("📂 应用状态已加载: {:?}", path);
                state
            }
            Err(e) => {
                log::warn!("⚠️ 加载状态失败: {}，使用默认状态", e);
                AppState::new()
            }
        }
    } else {
        log::info!("📂 未找到状态文件，创建新状态");
        AppState::new()
    }
}

fn save_app_state(state: &AppState) {
    let path = get_state_path();
    if let Err(e) = state.save_to_file(&path) {
        log::error!("❌ 保存状态失败: {}", e);
    } else {
        log::info!("💾 应用状态已保存: {:?}", path);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 异步注册流程
// ═══════════════════════════════════════════════════════════════════════════

/// 注册流程结果（包含 GyID 与位置信息）
#[derive(Debug, Clone)]
struct RegistrationResult {
    gy_id: String,
    city: String,
    address: String,
    lat: f64,
    lon: f64,
    timestamp: u64,
}

async fn run_registration(
    photo_path: PathBuf,
    app_weak: slint::Weak<MainWindow>,
) -> Result<RegistrationResult, Box<dyn std::error::Error + Send + Sync>> {
    // 步骤 0: 快速预检 — 照片格式 + GPS 可用性
    update_ui_location(&app_weak, "预检照片信息...");
    update_ui_progress(&app_weak, 0.05);
    tokio::task::yield_now().await; // 让 Slint 事件循环处理 UI 更新
    
    // 检查文件是否存在
    if !photo_path.exists() {
        let msg = format!("照片文件不存在: {}", photo_path.display());
        log::error!("{}", msg);
        update_ui_error(&app_weak, &msg);
        update_ui_location(&app_weak, "⚠️ 照片不存在");
        return Err(msg.into());
    }
    
    // 快速 GPS 预检（不需要读完整 EXIF，只检查 GPS 字段）
    log::info!("🔍 步骤0: GPS 预检 - {:?}", photo_path);
    let path_for_check = photo_path.clone();
    let check_result = tokio::task::spawn_blocking(move || {
        PhotoProcessor::check_gps(&path_for_check)
    }).await.map_err(|e| format!("GPS预检线程错误: {}", e))?;
    
    match check_result {
        Ok(Some(true)) => {
            log::info!("✅ 步骤0: GPS 预检通过");
        }
        Ok(Some(false)) => {
            let msg = "这张照片不包含 GPS 定位信息。\n\n请确保：\n· 使用手机原生相机拍摄\n· 拍照前已开启定位权限\n· 照片为 JPG 格式（非截图/非 PNG/非 HEIC）\n· 拍照时允许了相机访问位置的弹窗".to_string();
            log::warn!("⚠️ {}", msg.replace('\n', " "));
            update_ui_error(&app_weak, &msg);
            update_ui_location(&app_weak, "⚠️ 照片无 GPS 信息");
            update_ui_progress(&app_weak, 0.0);
            return Err(msg.into());
        }
        Ok(None) => {
            let msg = "无法读取照片元数据。\n\n可能原因：\n· 照片格式不支持（PNG/WebP/HEIC 不含 EXIF）\n· 照片被压缩或编辑过导致 EXIF 丢失\n· 文件损坏\n\n请使用手机原生相机拍摄 JPG 格式照片。".to_string();
            log::warn!("⚠️ {}", msg.replace('\n', " "));
            update_ui_error(&app_weak, &msg);
            update_ui_location(&app_weak, "⚠️ 无法读取元数据");
            update_ui_progress(&app_weak, 0.0);
            return Err(msg.into());
        }
        Err(e) => {
            let msg = format!("照片文件错误: {}", e);
            log::error!("{}", msg);
            update_ui_error(&app_weak, &msg);
            update_ui_location(&app_weak, "⚠️ 文件错误");
            update_ui_progress(&app_weak, 0.0);
            return Err(msg.into());
        }
    }

    // 步骤 1: 读取照片 EXIF
    log::info!("📷 步骤1: 读取 EXIF 元数据...");
    update_ui_location(&app_weak, "读取照片 EXIF 数据...");
    update_ui_progress(&app_weak, 0.15);
    tokio::task::yield_now().await;
    
    let path_for_meta = photo_path.clone();
    let metadata = tokio::task::spawn_blocking(move || {
        PhotoProcessor::extract_metadata(&path_for_meta)
    }).await.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
        format!("EXIF 解析线程错误: {}", e).into()
    })?.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
        log::error!("❌ 步骤1失败: EXIF 解析错误: {}", e);
        format!("照片元数据读取失败: {}。请确保照片为 JPG/JPEG 格式且包含 GPS 定位信息。", e).into()
    })?;
    
    let gps = match metadata.gps {
        Some(g) => g,
        None => {
            log::error!("❌ 步骤1失败: GPS 为 None（预检已通过）");
            return Err("照片缺少 GPS 信息（预检已通过但解析失败，请重试）。".into());
        }
    };
    log::info!("✅ 步骤1: EXIF 解析成功, GPS=({}, {}), timestamp={}", gps.latitude, gps.longitude, metadata.timestamp);
    update_ui_location(&app_weak, &format!(
        "GPS已获取 ({:.4}, {:.4})", 
        gps.latitude, gps.longitude
    ));
    update_ui_progress(&app_weak, 0.3);
    tokio::task::yield_now().await;

    // 步骤 2: 高德定位（可选，失败不阻塞流程）
    log::info!("🗺️ 步骤2: 高德逆地理编码...");
    update_ui_location(&app_weak, "高德逆地理编码...");
    update_ui_progress(&app_weak, 0.4);
    tokio::task::yield_now().await;
    
    let location_city: String;
    let location_address: String;

    let service = LocationService::default();
    match tokio::time::timeout(
        std::time::Duration::from_secs(3), // 缩短到 3 秒（API 正常响应 < 1s）
        service.verify_and_enhance(gps.clone())
    ).await {
        Ok(Ok(location)) => {
            log::info!("✅ 步骤2: 高德定位成功, address={:?}, accuracy={}", 
                location.address, location.accuracy);
            location_city = location.city.clone().unwrap_or_else(|| "未知城市".to_string());
            location_address = location.address.clone().unwrap_or_else(|| format!("GPS坐标 ({:.4}, {:.4})", gps.latitude, gps.longitude));
            update_ui_location(&app_weak, location.address.as_deref().unwrap_or("定位成功"));
            update_ui_progress(&app_weak, 0.6);
        }
        Ok(Err(e)) => {
            // 高德 API 返回错误（如 INVALID_USER_IP），降级
            log::warn!("⚠️ 步骤2: 高德定位失败(降级): {} — 使用原始 GPS 坐标继续", e);
            location_city = "未知城市".to_string();
            location_address = format!("GPS坐标 ({:.4}, {:.4})", gps.latitude, gps.longitude);
            update_ui_location(&app_weak, &location_address);
            update_ui_progress(&app_weak, 0.6);
        }
        Err(_) => {
            // 3 秒超时，降级
            log::warn!("⚠️ 步骤2: 高德定位超时(降级) — 使用原始 GPS 坐标继续");
            location_city = "未知城市".to_string();
            location_address = format!("GPS坐标 ({:.4}, {:.4})", gps.latitude, gps.longitude);
            update_ui_location(&app_weak, &location_address);
            update_ui_progress(&app_weak, 0.6);
        }
    }
    tokio::task::yield_now().await;

    // 步骤 3: 生成 GyID
    log::info!("🆔 步骤3: 生成 GyID...");
    update_ui_location(&app_weak, "生成 GyID 身份标识...");
    update_ui_progress(&app_weak, 0.7);
    tokio::task::yield_now().await;
    
    let path_for_gyid = photo_path.clone();
    let lat = gps.latitude;
    let lon = gps.longitude;
    let ts = metadata.timestamp;
    let gy_id = tokio::task::spawn_blocking(move || {
        CryptoService::generate_gyid(&path_for_gyid, lat, lon, ts)
    }).await.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
        log::error!("❌ 步骤3: GyID 生成线程错误: {}", e);
        format!("GyID 生成线程错误: {}", e).into()
    })?.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
        log::error!("❌ 步骤3失败: GyID 生成错误: {}", e);
        format!("GyID 生成失败: {}", e).into()
    })?;
    log::info!("✅ 步骤3: GyID 生成成功: {}", gy_id);
    update_ui_location(&app_weak, "GyID 已生成!");
    update_ui_progress(&app_weak, 0.9);
    tokio::task::yield_now().await;

    // 步骤 4: 铸造
    log::info!("🪙 铸造首枚 Geoyuan...");
    update_ui_location(&app_weak, "铸造首枚 Geoyuan...");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    Ok(RegistrationResult {
        gy_id,
        city: location_city,
        address: location_address,
        lat: gps.latitude,
        lon: gps.longitude,
        timestamp: metadata.timestamp as u64,
    })
}
