//! C FFI 接口
//!
//! 提供标准 C ABI，可被 iOS Swift、Python、C/C++、Unity 等调用。
//!
//! ## Swift 调用示例（iOS）
//! ```swift
//! // 通过 bridging header 引入
//! let result = gyid_generate_default()
//! if let cStr = result {
//!     let json = String(cString: cStr)
//!     gyid_free_string(result)
//! }
//! ```

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::models::GenerateOptions;

/// 结果状态码
#[repr(C)]
pub enum GyIdStatus {
    Ok = 0,
    ErrorFingerprint = 1,
    ErrorGeo = 2,
    ErrorAvatar = 3,
    ErrorCrypto = 4,
    ErrorStorage = 5,
    ErrorInvalidParam = 6,
    ErrorPermission = 7,
    ErrorNetwork = 8,
    ErrorPlatformNotSupported = 9,
}

/// C FFI 结果结构
#[repr(C)]
pub struct GyIdCResult {
    pub status: GyIdStatus,
    /// JSON 字符串，需要调用 gyid_free_string 释放
    pub data: *mut c_char,
    /// 错误信息（仅在 status != Ok 时有效）
    pub error_message: *mut c_char,
}

impl GyIdCResult {
    fn ok(data: String) -> Self {
        let c_data = CString::new(data).unwrap_or_default();
        Self {
            status: GyIdStatus::Ok,
            data: c_data.into_raw(),
            error_message: std::ptr::null_mut(),
        }
    }

    fn err(status: GyIdStatus, message: String) -> Self {
        let c_msg = CString::new(message).unwrap_or_default();
        Self {
            status,
            data: std::ptr::null_mut(),
            error_message: c_msg.into_raw(),
        }
    }
}

/// 释放由 SDK 分配的 C 字符串内存
///
/// # Safety
/// `ptr` 必须是由本 SDK 分配的 CString 原始指针，或 NULL。每个 SDK 返回的字符串只能释放一次。
#[no_mangle]
pub unsafe extern "C" fn gyid_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(ptr));
    }
}

/// 释放 GyIdCResult 结构中的内存
///
/// # Safety
/// `result` 的内部指针字段必须是由本 SDK 分配的，且未被释放过。
#[no_mangle]
pub unsafe extern "C" fn gyid_free_result(result: GyIdCResult) {
    gyid_free_string(result.data);
    gyid_free_string(result.error_message);
}

/// 使用默认配置生成 GyID（阻塞调用，适合非 WASM 原生平台）
///
/// 返回的 GyIdCResult.data 是 JSON 字符串，使用后需调用 gyid_free_result 释放
#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub extern "C" fn gyid_generate_default() -> GyIdCResult {
    use tokio::runtime::Runtime;
    use crate::api::GyIdSdk;

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            return GyIdCResult::err(
                GyIdStatus::ErrorCrypto,
                format!("Tokio runtime 创建失败: {}", e),
            )
        }
    };

    match rt.block_on(GyIdSdk::generate_default()) {
        Ok(result) => {
            let json = serde_json::to_string(&result).unwrap_or_default();
            GyIdCResult::ok(json)
        }
        Err(e) => GyIdCResult::err(GyIdStatus::ErrorCrypto, e.to_string()),
    }
}

/// 使用 JSON 选项生成 GyID（阻塞调用）
///
/// # Safety
/// `options_json` 必须是有效的以 null 结尾的 UTF-8 C 字符串指针，或 NULL。
///
/// @param options_json - 选项 JSON C 字符串（UTF-8）
#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn gyid_generate(options_json: *const c_char) -> GyIdCResult {
    use tokio::runtime::Runtime;
    use crate::api::GyIdSdk;

    if options_json.is_null() {
        return gyid_generate_default();
    }

    let json_str = unsafe {
        match CStr::from_ptr(options_json).to_str() {
            Ok(s) => s,
            Err(e) => {
                return GyIdCResult::err(
                    GyIdStatus::ErrorInvalidParam,
                    format!("options_json 不是合法的 UTF-8: {}", e),
                )
            }
        }
    };

    let opts: GenerateOptions = serde_json::from_str(json_str)
        .unwrap_or_else(|_| GenerateOptions::default());

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            return GyIdCResult::err(
                GyIdStatus::ErrorCrypto,
                format!("Tokio runtime 创建失败: {}", e),
            )
        }
    };

    match rt.block_on(GyIdSdk::generate(opts)) {
        Ok(result) => {
            let json = serde_json::to_string(&result).unwrap_or_default();
            GyIdCResult::ok(json)
        }
        Err(e) => GyIdCResult::err(GyIdStatus::ErrorCrypto, e.to_string()),
    }
}

/// 验证 GyID 格式
///
/// # Safety
/// `gyid_str` 必须是有效的以 null 结尾的 UTF-8 C 字符串指针，或 NULL。
///
/// @param gyid_str - GyID C 字符串
/// @returns 1 = 有效, 0 = 无效或错误
#[no_mangle]
pub unsafe extern "C" fn gyid_verify(gyid_str: *const c_char) -> i32 {
    use crate::api::GyIdSdk;

    if gyid_str.is_null() {
        return 0;
    }
    let s = unsafe {
        match CStr::from_ptr(gyid_str).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };
    if GyIdSdk::verify(s) { 1 } else { 0 }
}

/// 获取 SDK 版本信息 JSON 字符串
///
/// 返回值需调用 gyid_free_string 释放
#[no_mangle]
pub extern "C" fn gyid_version() -> *mut c_char {
    use crate::api::GyIdSdk;
    let version = serde_json::to_string(&GyIdSdk::version()).unwrap_or_default();
    CString::new(version).unwrap_or_default().into_raw()
}

/// 获取关联设备列表
///
/// # Safety
/// `gyid_str` 必须是有效的以 null 结尾的 UTF-8 C 字符串指针，或 NULL。
///
/// @param gyid_str - 主设备 GyID（可为 NULL，使用本地保存的）
/// @returns GyIdCResult，data 为 DeviceList JSON
#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn gyid_get_devices(gyid_str: *const c_char) -> GyIdCResult {
    use crate::api::GyIdSdk;

    let gyid_opt = if gyid_str.is_null() {
        None
    } else {
        unsafe {
            CStr::from_ptr(gyid_str).to_str().ok().map(|s| s.to_string())
        }
    };

    match GyIdSdk::get_devices(gyid_opt.as_deref()) {
        Ok(list) => {
            let json = serde_json::to_string(&list).unwrap_or_default();
            GyIdCResult::ok(json)
        }
        Err(e) => GyIdCResult::err(GyIdStatus::ErrorStorage, e.to_string()),
    }
}

/// 导出 GyID 为 JSON
///
/// @returns GyIdCResult，data 为 GyIdResult JSON
#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub extern "C" fn gyid_export_json() -> GyIdCResult {
    use crate::api::GyIdSdk;

    match GyIdSdk::export_json() {
        Ok(json) => GyIdCResult::ok(json),
        Err(e) => GyIdCResult::err(GyIdStatus::ErrorStorage, e.to_string()),
    }
}
