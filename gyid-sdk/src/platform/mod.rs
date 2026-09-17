//! 平台特定模块
//!
//! 根据目标平台条件编译不同的实现

pub mod fingerprint;

#[cfg(target_arch = "wasm32")]
pub mod wasm_compat;

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
