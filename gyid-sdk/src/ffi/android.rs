//! Android JNI 绑定
//!
//! 提供 Kotlin/Java 可调用的 JNI 接口
//!
//! ## Kotlin 调用示例
//! ```kotlin
//! object GyIdLib {
//!     init { System.loadLibrary("gyid_sdk") }
//!
//!     external fun generateDefault(): String
//!     external fun generate(optionsJson: String): String
//!     external fun verify(gyid: String): Boolean
//!     external fun getVersion(): String
//! }
//!
//! // 使用
//! val result = GyIdLib.generateDefault()
//! val gyid = JSONObject(result)
//! println(gyid.getString("id"))
//! ```

#[cfg(all(target_os = "android", feature = "android"))]
use jni::{
    JNIEnv,
    objects::{JClass, JString},
    sys::{jboolean, jstring},
};

#[cfg(all(target_os = "android", feature = "android"))]
use crate::api::GyIdSdk;
#[cfg(all(target_os = "android", feature = "android"))]
use crate::models::GenerateOptions;

/// JNI: 使用默认配置生成 GyID
#[no_mangle]
#[cfg(all(target_os = "android", feature = "android"))]
pub extern "system" fn Java_io_gyid_sdk_GyIdLib_generateDefault(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    use tokio::runtime::Runtime;

    let rt = Runtime::new().expect("Tokio runtime failed");
    let result = rt.block_on(GyIdSdk::generate_default());

    match result {
        Ok(gyid) => {
            let json = serde_json::to_string(&gyid).unwrap_or_default();
            env.new_string(json)
                .expect("JNI string creation failed")
                .into_raw()
        }
        Err(e) => {
            let err_json = serde_json::json!({ "error": e.to_string() }).to_string();
            env.new_string(err_json)
                .expect("JNI string creation failed")
                .into_raw()
        }
    }
}

/// JNI: 自定义配置生成 GyID
#[no_mangle]
#[cfg(all(target_os = "android", feature = "android"))]
pub extern "system" fn Java_io_gyid_sdk_GyIdLib_generate(
    mut env: JNIEnv,
    _class: JClass,
    options_json: JString,
) -> jstring {
    use tokio::runtime::Runtime;

    let opts_str: String = env
        .get_string(&options_json)
        .map(|s| s.into())
        .unwrap_or_default();

    let opts: GenerateOptions = serde_json::from_str(&opts_str)
        .unwrap_or_else(|_| GenerateOptions::default());

    let rt = Runtime::new().expect("Tokio runtime failed");
    let result = rt.block_on(GyIdSdk::generate(opts));

    match result {
        Ok(gyid) => {
            let json = serde_json::to_string(&gyid).unwrap_or_default();
            env.new_string(json)
                .expect("JNI string creation failed")
                .into_raw()
        }
        Err(e) => {
            let err_json = serde_json::json!({ "error": e.to_string() }).to_string();
            env.new_string(err_json)
                .expect("JNI string creation failed")
                .into_raw()
        }
    }
}

/// JNI: 验证 GyID 格式
#[no_mangle]
#[cfg(all(target_os = "android", feature = "android"))]
pub extern "system" fn Java_io_gyid_sdk_GyIdLib_verify(
    mut env: JNIEnv,
    _class: JClass,
    gyid_jstr: JString,
) -> jboolean {
    let gyid_str: String = env
        .get_string(&gyid_jstr)
        .map(|s| s.into())
        .unwrap_or_default();

    if GyIdSdk::verify(&gyid_str) { 1 } else { 0 }
}

/// JNI: 获取 SDK 版本
#[no_mangle]
#[cfg(all(target_os = "android", feature = "android"))]
pub extern "system" fn Java_io_gyid_sdk_GyIdLib_getVersion(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let version = serde_json::to_string(&GyIdSdk::version()).unwrap_or_default();
    env.new_string(version)
        .expect("JNI string creation failed")
        .into_raw()
}

/// JNI: 获取关联设备列表
#[no_mangle]
#[cfg(all(target_os = "android", feature = "android"))]
pub extern "system" fn Java_io_gyid_sdk_GyIdLib_getDevices(
    mut env: JNIEnv,
    _class: JClass,
    master_gyid: JString,
) -> jstring {
    use tokio::runtime::Runtime;

    let master_str: String = env
        .get_string(&master_gyid)
        .map(|s| s.into())
        .unwrap_or_default();

    let master_opt = if master_str.is_empty() { None } else { Some(master_str.as_str()) };

    let rt = Runtime::new().expect("Tokio runtime failed");
    let result = rt.block_on(async {
        GyIdSdk::get_devices(master_opt)
    });

    match result {
        Ok(list) => {
            let json = serde_json::to_string(&list).unwrap_or_default();
            env.new_string(json)
                .expect("JNI string creation failed")
                .into_raw()
        }
        Err(e) => {
            let err_json = serde_json::json!({ "error": e.to_string() }).to_string();
            env.new_string(err_json)
                .expect("JNI string creation failed")
                .into_raw()
        }
    }
}

/// JNI: 导出 GyID 为 JSON
#[no_mangle]
#[cfg(all(target_os = "android", feature = "android"))]
pub extern "system" fn Java_io_gyid_sdk_GyIdLib_exportJson(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    match GyIdSdk::export_json() {
        Ok(json) => env.new_string(json)
            .expect("JNI string creation failed")
            .into_raw(),
        Err(e) => {
            let err_json = serde_json::json!({ "error": e.to_string() }).to_string();
            env.new_string(err_json)
                .expect("JNI string creation failed")
                .into_raw()
        }
    }
}
