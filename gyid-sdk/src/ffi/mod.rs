//! FFI 绑定模块
//!
//! 按目标平台提供不同的语言绑定：
//! - WASM: wasm-bindgen → JavaScript/TypeScript
//! - Android: JNI → Kotlin/Java
//! - iOS/通用: C FFI → Swift/任意语言

// WASM 绑定
#[cfg(target_arch = "wasm32")]
pub mod wasm;

// Android JNI 绑定（需要 feature = "android"）
#[cfg(all(target_os = "android", feature = "android"))]
pub mod android;

// C FFI（通用，可用于 iOS、C、Python 等）
pub mod c_ffi;

// UniFFI 绑定定义（生成 Swift/Kotlin 绑定）
#[cfg(feature = "uniffi-bindings")]
uniffi::setup_scaffolding!();
