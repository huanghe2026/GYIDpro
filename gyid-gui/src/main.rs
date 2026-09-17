// main.rs — GyID Slint GUI 入口

mod state;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use arboard::Clipboard;
use rfd::FileDialog;
use slint::{include_modules, ComponentHandle, SharedString, VecModel};
use state::{AppSettings, AppState, GenStatus, GeoPrecision};

// ── 导出辅助函数 ────────────────────────────────────────
fn copy_to_clipboard(text: &str) -> bool {
    let mut clipboard = Clipboard::new().ok();
    if let Some(ref mut cb) = clipboard {
        cb.set_text(text.to_string()).is_ok()
    } else {
        false
    }
}

fn export_gyid_json(gyid: &str) -> Option<PathBuf> {
    let json = serde_json::json!({
        "id": gyid,
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "version": "0.4.1"
    });

    let path = FileDialog::new()
        .set_title("导出 GyID")
        .set_file_name("gyid.json")
        .add_filter("JSON 文件", &["json"])
        .save_file()?;

    let mut file = File::create(&path).ok()?;
    let mut writer = BufWriter::new(&mut file);
    writer.write_all(serde_json::to_string_pretty(&json).ok()?.as_bytes()).ok()?;
    writer.flush().ok()?;
    Some(path)
}

slint::include_modules!();

// ── 辅助函数 ────────────────────────────────────────
fn to_slint(s: &str) -> SharedString {
    SharedString::from(s)
}

fn update_ui_from_state(app: &AppWindow, state: &AppState) {
    let status = state.generation_status.lock().unwrap();
    let settings = state.settings.lock().unwrap();
    
    // 更新状态文本
    let text = match &*status {
        GenStatus::Idle => "就绪".to_string(),
        GenStatus::CollectingHardware => "采集中: 硬件指纹...".to_string(),
        GenStatus::CollectingGeo => "采集中: 地理位置...".to_string(),
        GenStatus::Hashing => "计算中: 哈希生成...".to_string(),
        GenStatus::Done(_) => "生成完成".to_string(),
        GenStatus::Error(e) => format!("错误: {}", e),
        GenStatus::Cancelled => "已取消".to_string(),
        GenStatus::Paused => "已暂停".to_string(),
        GenStatus::Timeout(_) => "超时".to_string(),
    };
    
    app.set_status_text(to_slint(&text));
    app.set_is_generating(status.is_running());
    
    // 同步设置
    app.set_geo_level(match settings.geo_precision {
        GeoPrecision::City => 0,
        GeoPrecision::District => 1,
        GeoPrecision::Exact => 2,
    });
    app.set_use_avatar(settings.use_avatar);
    
    // 同步历史
    let history_model = VecModel::<HistoryItem>::default();
    for item in settings.history() {
        history_model.push(HistoryItem {
            id: item.id.clone().into(),
            time: item.time.clone().into(),
            geo_level: item.geo_level.clone().into(),
        });
    }
    app.set_history(std::rc::Rc::new(history_model).into());
}

// ── 主函数 ───────────────────────────────────────────
fn main() {
    // 加载设置
    let settings = AppSettings::load().unwrap_or_default();
    let state = Arc::new(AppState::new(settings));
    
    // 创建窗口
    let app = AppWindow::new().unwrap();
    
    // 初始化 UI 状态
    update_ui_from_state(&app, &state);
    
    // ── 回调: 生成 ──────────────────────────────────
    let state_clone = state.clone();
    app.on_generate(move || {
        let state_arc = state_clone.clone();
        
        // 更新状态为采集中
        {
            let mut status = state_arc.generation_status.lock().unwrap();
            *status = GenStatus::CollectingHardware;
        }
        
        // 后台线程生成
        thread::spawn(move || {
            let config = {
                let s = state_arc.settings.lock().unwrap();
                let precision = match s.geo_precision {
                    GeoPrecision::City => gyid_core::geo::GeoPrecisionLevel::City,
                    GeoPrecision::District => gyid_core::geo::GeoPrecisionLevel::District,
                    GeoPrecision::Exact => gyid_core::geo::GeoPrecisionLevel::Exact,
                };
                let use_avatar = s.use_avatar;
                gyid_core::identity::GeneratorConfig {
                    geo_level: precision,
                    weights: gyid_core::crypto::GyIdWeights::default(),
                    with_avatar: use_avatar,
                    with_geo: true,
                }
            };
            
            // 更新为地理位置采集
            {
                let mut status = state_arc.generation_status.lock().unwrap();
                *status = GenStatus::CollectingGeo;
            }
            
            // 更新为哈希计算
            {
                let mut status = state_arc.generation_status.lock().unwrap();
                *status = GenStatus::Hashing;
            }
            
            // 执行生成
            let result = {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    let generator = gyid_core::identity::GyIdGenerator::new(config);
                    let avatar_path = state_arc.settings.lock().unwrap().avatar_path.clone();
                    generator.generate(avatar_path.as_deref()).await
                })
            };
            
            match result {
                Ok(gyid) => {
                    let gyid_str = gyid.id.clone();
                    
                    // 保存历史
                    {
                        let mut settings = state_arc.settings.lock().unwrap();
                        let geo_level = settings.geo_precision.as_str().to_string();
                        let has_avatar = settings.use_avatar;
                        settings.add_to_history(gyid_str.clone(), geo_level, has_avatar);
                        let _ = settings.save();
                    }
                    
                    // 更新状态
                    {
                        let mut status = state_arc.generation_status.lock().unwrap();
                        *status = GenStatus::Done(gyid_str);
                    }
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    {
                        let mut status = state_arc.generation_status.lock().unwrap();
                        *status = GenStatus::Error(err_msg);
                    }
                }
            }
        });
    });
    
    // ── 回调: 取消 ─────────────────────────────────
    let state_clone = state.clone();
    app.on_cancel(move || {
        let mut cancel = state_clone.cancel_flag.lock().unwrap();
        *cancel = true;
        
        let mut status = state_clone.generation_status.lock().unwrap();
        if let GenStatus::CollectingHardware | GenStatus::CollectingGeo | GenStatus::Hashing = *status {
            *status = GenStatus::Cancelled;
        }
    });
    
    // ── 回调: 改变地理精度 ─────────────────────────
    let state_clone = state.clone();
    app.on_change_geo_level(move |level| {
        let precision = match level {
            0 => GeoPrecision::City,
            1 => GeoPrecision::District,
            _ => GeoPrecision::Exact,
        };
        
        let mut settings = state_clone.settings.lock().unwrap();
        settings.geo_precision = precision;
        let _ = settings.save();
    });
    
    // ── 回调: 切换头像 ─────────────────────────────
    let state_clone = state.clone();
    app.on_toggle_avatar(move |enabled| {
        let mut settings = state_clone.settings.lock().unwrap();
        settings.use_avatar = enabled;
        let _ = settings.save();
    });
    
    // ── 回调: 复制 GyID ────────────────────────────
    let app_handle = app.as_weak();
    app.on_export_qr(move |gyid| {
        if copy_to_clipboard(&gyid) {
            if let Some(app) = app_handle.upgrade() {
                app.set_status_text(to_slint("已复制到剪贴板"));
            }
        }
    });
    
    // ── 回调: 导出 JSON ────────────────────────────
    let app_handle = app.as_weak();
    app.on_export_json(move |gyid| {
        if let Some(path) = export_gyid_json(&gyid) {
            if let Some(app) = app_handle.upgrade() {
                app.set_status_text(to_slint(&format!("已导出: {}", path.file_name().unwrap_or_default().to_string_lossy())));
            }
        }
    });
    
    // ── 状态轮询（定期更新 UI）─────────────────────
    // 使用定时器宏
    let state_poll = state.clone();
    let app_poll = app.as_weak();
    
    let _timer = slint::Timer::default();
    _timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(200), move || {
        if let Some(app) = app_poll.upgrade() {
            let status = state_poll.generation_status.lock().unwrap();
            let settings = state_poll.settings.lock().unwrap();
            
            // 更新状态文本
            let text = match &*status {
                GenStatus::Idle => "就绪".to_string(),
                GenStatus::CollectingHardware => "采集中: 硬件指纹...".to_string(),
                GenStatus::CollectingGeo => "采集中: 地理位置...".to_string(),
                GenStatus::Hashing => "计算中: 哈希生成...".to_string(),
                GenStatus::Done(ref id) => {
                    app.set_gyid_result(to_slint(id));
                    app.set_has_result(true);
                    "生成完成".to_string()
                }
                GenStatus::Error(ref e) => format!("错误: {}", e),
                GenStatus::Cancelled => "已取消".to_string(),
                GenStatus::Paused => "已暂停".to_string(),
                GenStatus::Timeout(_) => "超时".to_string(),
            };
            
            app.set_status_text(to_slint(&text));
            app.set_is_generating(status.is_running());
            app.set_geo_level(match settings.geo_precision {
                GeoPrecision::City => 0,
                GeoPrecision::District => 1,
                GeoPrecision::Exact => 2,
            });
            app.set_use_avatar(settings.use_avatar);
        }
    });
    
    // 运行
    app.run().unwrap();
}
