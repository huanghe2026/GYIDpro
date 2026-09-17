//! 跨平台硬件指纹抽象
//!
//! 在 WASM 平台，硬件指纹通过 JS 环境特征（navigator, screen, canvas）采集。
//! 在原生平台，直接调用 gyid-core 的 fingerprint 模块。

use crate::error::SdkResult;

/// 平台指纹采集器
pub struct PlatformFingerprint;

impl PlatformFingerprint {
    /// 采集当前平台的硬件/环境指纹字符串
    ///
    /// 返回用于参与 GyID 计算的指纹字符串列表
    pub fn collect() -> SdkResult<Vec<String>> {
        #[cfg(target_arch = "wasm32")]
        return Self::collect_wasm();

        #[cfg(not(target_arch = "wasm32"))]
        return Self::collect_native();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn collect_native() -> SdkResult<Vec<String>> {
        use gyid_core::fingerprint::{MacCollector, CpuCollector, BoardCollector, DiskCollector};
        use crate::error::SdkError;

        let mut factors = Vec::new();

        if let Ok(Some(mac)) = MacCollector::collect() {
            factors.push(mac);
        }
        if let Ok(Some(cpu)) = CpuCollector::collect() {
            factors.push(cpu);
        }
        if let Ok(Some(board)) = BoardCollector::collect() {
            factors.push(board);
        }
        if let Ok(Some(disk)) = DiskCollector::collect() {
            factors.push(disk);
        }

        if factors.is_empty() {
            return Err(SdkError::Fingerprint(
                "无法采集到任何硬件指纹因子".to_string()
            ));
        }

        Ok(factors)
    }

    #[cfg(target_arch = "wasm32")]
    fn collect_wasm() -> SdkResult<Vec<String>> {
        // WASM 平台：通过 JS 环境特征采集
        // 注意：WASM 中无法访问真实硬件，使用软件指纹替代
        use web_sys::window;

        let mut factors = Vec::new();

        if let Some(win) = window() {
            let nav = win.navigator();

            // 用户代理
            if let Ok(ua) = nav.user_agent() {
                if !ua.is_empty() {
                    factors.push(format!("ua:{}", ua));
                }
            }

            // 平台
            #[allow(deprecated)]
            if let Ok(platform) = nav.platform() {
                if !platform.is_empty() {
                    factors.push(format!("platform:{}", platform));
                }
            }

            // 语言
            if let Some(lang) = nav.language() {
                factors.push(format!("lang:{}", lang));
            }

            // 硬件并发数（CPU 核心数）
            let hw_concurrency = nav.hardware_concurrency();
            if hw_concurrency > 0.0 {
                factors.push(format!("cpu_cores:{}", hw_concurrency as u32));
            }

            // 屏幕信息
            if let Ok(screen) = win.screen() {
                let width = screen.width().unwrap_or(0);
                let height = screen.height().unwrap_or(0);
                let depth = screen.color_depth().unwrap_or(0);
                factors.push(format!("screen:{}x{}x{}", width, height, depth));
            }

            // 时区偏移
            if let Ok(tz) = js_sys::Reflect::get(
                &js_sys::Date::new_0(),
                &wasm_bindgen::JsValue::from_str("getTimezoneOffset"),
            ) {
                factors.push(format!("tz:{:?}", tz));
            }
        }

        if factors.is_empty() {
            // 最低保底：使用随机 UUID（仅在完全无法采集时）
            let random: u64 = (js_sys::Math::random() * u64::MAX as f64) as u64;
            factors.push(format!("random:{}", random));
        }

        Ok(factors)
    }
}
