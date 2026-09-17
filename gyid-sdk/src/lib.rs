//! # GyID SDK
//!
//! 跨平台去中心化身份 SDK，支持：
//! - **桌面端**: Windows / macOS / Linux (Rust native)
//! - **Web 端**: WebAssembly (wasm-bindgen)  
//! - **Android**: JNI 绑定
//! - **iOS / macOS**: UniFFI → Swift / Kotlin
//!
//! ## 快速开始
//!
//! ### Rust 端
//! ```rust,ignore
//! use gyid_sdk::GyIdSdk;
//! let result = GyIdSdk::generate_default().await.unwrap();
//! println!("GyID: {}", result.id);
//! ```
//!
//! ### JavaScript/TypeScript (WASM)
//! ```js
//! import init, { generate_gyid } from '@gyid/sdk-web';
//! await init();
//! const result = await generate_gyid({ geo_level: 'city' });
//! console.log(result.id);
//! ```

pub mod api;
pub mod error;
pub mod models;

// 平台特定模块
pub mod platform;

// FFI 绑定模块（按目标平台条件编译）
pub mod ffi;

// 重新导出核心类型
pub use api::GyIdSdk;
pub use error::SdkError;
pub use models::{GenerateOptions, GyIdResult, SdkVersion};

/// SDK 版本信息
pub const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");
/// SDK 名称
pub const SDK_NAME: &str = "GyID SDK";
