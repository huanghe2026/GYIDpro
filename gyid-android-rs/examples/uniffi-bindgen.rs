//! UniFFI bindgen 薄入口（UniFFI 0.28 不再发布独立二进制）。
//!
//! 用法：
//! ```text
//! cargo run -p gyid-android-rs --example uniffi-bindgen -- \
//!   generate --language kotlin --out-dir kotlin src/gyid.udl
//! ```

fn main() {
    uniffi::uniffi_bindgen_main();
}
