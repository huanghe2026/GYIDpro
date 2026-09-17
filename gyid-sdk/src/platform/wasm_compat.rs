//! WASM 平台兼容层
//!
//! 为 WASM 目标提供原生平台 API 的替代实现

/// WASM 平台的时间戳获取
#[cfg(target_arch = "wasm32")]
pub fn now_millis() -> u64 {
    js_sys::Date::now() as u64
}

/// WASM 平台的随机数生成
#[cfg(target_arch = "wasm32")]
pub fn random_u16() -> u16 {
    (js_sys::Math::random() * 65535.0) as u16
}
