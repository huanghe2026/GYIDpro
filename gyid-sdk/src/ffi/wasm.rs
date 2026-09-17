//! WASM 绑定 (wasm-bindgen)
//!
//! 将 GyID SDK 暴露为 JavaScript/TypeScript 可调用的接口
//!
//! ## 使用方式（JavaScript）
//! ```js
//! import init, { generate_gyid, verify_gyid, get_sdk_version } from '@gyid/sdk-web';
//!
//! await init(); // 初始化 WASM 模块
//!
//! // 生成 GyID
//! const result = await generate_gyid(JSON.stringify({ geo_level: "city" }));
//! const gyid = JSON.parse(result);
//! console.log(gyid.id); // "GyID..."
//!
//! // 验证 GyID
//! const isValid = verify_gyid(gyid.id); // true/false
//! ```

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::future_to_promise;

#[cfg(target_arch = "wasm32")]
use crate::api::GyIdSdk;

#[cfg(target_arch = "wasm32")]
use crate::models::GenerateOptions;

/// 初始化 WASM 模块（设置 panic hook）
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_init() {
    // 设置 panic 时将详细信息输出到浏览器控制台
    console_error_panic_hook::set_once();
}

/// 生成 GyID
///
/// @param options_json - JSON 字符串，格式: `{ "geo_level": "city" | "district" | "exact", "with_geo": true }`
/// @returns Promise<string> - JSON 字符串，包含 GyID 信息
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn generate_gyid(options_json: &str) -> js_sys::Promise {
    let opts: GenerateOptions = serde_json::from_str(options_json)
        .unwrap_or_else(|_| GenerateOptions::default());

    future_to_promise(async move {
        match GyIdSdk::generate(opts).await {
            Ok(result) => {
                let json = serde_json::to_string(&result)
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
                Ok(JsValue::from_str(&json))
            }
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    })
}

/// 使用默认配置生成 GyID
///
/// @returns Promise<string> - JSON 字符串
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn generate_gyid_default() -> js_sys::Promise {
    future_to_promise(async move {
        match GyIdSdk::generate_default().await {
            Ok(result) => {
                let json = serde_json::to_string(&result)
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
                Ok(JsValue::from_str(&json))
            }
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    })
}

/// 验证 GyID 格式
///
/// @param gyid_str - GyID 字符串
/// @returns boolean - true 表示格式正确
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn verify_gyid(gyid_str: &str) -> bool {
    GyIdSdk::verify(gyid_str)
}

/// 获取 SDK 版本信息
///
/// @returns string - JSON 字符串，包含版本和特性信息
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_sdk_version() -> String {
    serde_json::to_string(&GyIdSdk::version()).unwrap_or_default()
}

/// 哈希字符串（工具函数，可用于前端调用 BLAKE3）
///
/// @param input - 输入字符串
/// @returns string - BLAKE3 哈希的十六进制字符串（64字符）
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn hash_string(input: &str) -> String {
    use gyid_core::crypto::GyIdHasher;
    GyIdHasher::hash_strings(&[input])
}
