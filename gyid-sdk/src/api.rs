//! SDK 核心 API
//!
//! 跨平台统一接口，内部调用 gyid-core 实现。
//! 非 WASM 平台使用 tokio 异步运行时；
//! WASM 平台通过 wasm-bindgen-futures 映射到 Promise。

use crate::{
    error::{SdkError, SdkResult},
    models::{
        GenerateOptions, GyIdResult, SdkVersion, WeightsOptions, current_platform,
    },
    SDK_VERSION,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::models::{AuthCodeResult, DeviceInfo, DeviceList, LinkOptions, LinkResult};

use gyid_core::{
    GeneratorConfig, GyIdGenerator,
    geo::GeoPrecisionLevel,
    crypto::GyIdWeights,
    crypto::GyIdHasher,
};
#[cfg(not(target_arch = "wasm32"))]
use gyid_core::{
    storage::LocalStorage,
    device::DeviceLink,
};
use chrono::Utc;

/// GyID SDK 主接口
///
/// 所有功能通过此结构体暴露，设计为无状态（依赖 LocalStorage 持久化）。
pub struct GyIdSdk;

impl GyIdSdk {
    // ───────────────────────────────────────────────────────────────────────────
    // ① 生成 GyID
    // ───────────────────────────────────────────────────────────────────────────

    /// 使用默认配置生成 GyID（城市级地理位置，无头像）
    pub async fn generate_default() -> SdkResult<GyIdResult> {
        Self::generate(GenerateOptions::default()).await
    }

    /// 使用自定义配置生成 GyID
    pub async fn generate(opts: GenerateOptions) -> SdkResult<GyIdResult> {
        let geo_level = parse_geo_level(opts.geo_level.as_deref().unwrap_or("city"))?;
        let weights = opts.weights.as_ref().map(parse_weights).unwrap_or_else(|| Ok(GyIdWeights::default()))?;
        let with_geo = opts.with_geo.unwrap_or(true);
        let avatar_path = opts.avatar_path.as_deref();

        // 处理内存中的头像字节
        // 当提供 avatar_bytes 时，写入临时文件（非 WASM）或跳过（WASM 暂不支持）
        #[cfg(not(target_arch = "wasm32"))]
        let tmp_avatar: Option<tempfile::NamedTempFile> = if let Some(bytes) = opts.avatar_bytes {
            if !bytes.is_empty() {
                let tmp = tempfile::Builder::new()
                    .suffix(".png")
                    .tempfile()
                    .map_err(|e| SdkError::Avatar(format!("临时文件创建失败: {}", e)))?;
                std::fs::write(tmp.path(), &bytes)
                    .map_err(|e| SdkError::Avatar(format!("写入临时文件失败: {}", e)))?;
                Some(tmp)
            } else {
                None
            }
        } else {
            None
        };

        // 确定最终头像路径
        #[cfg(not(target_arch = "wasm32"))]
        let final_avatar_path: Option<String> = tmp_avatar
            .as_ref()
            .map(|f: &tempfile::NamedTempFile| f.path().to_string_lossy().to_string())
            .or_else(|| avatar_path.map(|p| p.to_string()));

        #[cfg(target_arch = "wasm32")]
        let final_avatar_path: Option<String> = avatar_path.map(|p| p.to_string());

        let config = GeneratorConfig {
            geo_level,
            weights,
            with_avatar: final_avatar_path.is_some(),
            with_geo,
        };

        let generator = GyIdGenerator::new(config);
        let gyid = generator
            .generate(final_avatar_path.as_deref())
            .await
            .map_err(SdkError::from)?;

        // 自动持久化（非 WASM 平台）
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(storage) = LocalStorage::new(None) {
                let _ = storage.save_gyid(&gyid);
                let _ = storage.save_config("last_gyid", &gyid.id);
            }
        }

        let created_at_ms = (gyid.created_at >> 16) as i64;

        Ok(GyIdResult {
            id: gyid.id,
            hash: gyid.hash,
            created_at_ms,
            linked_devices: gyid.linked_devices,
            platform: current_platform().to_string(),
            sdk_version: SDK_VERSION.to_string(),
        })
    }

    // ───────────────────────────────────────────────────────────────────────────
    // ② 验证 GyID
    // ───────────────────────────────────────────────────────────────────────────

    /// 验证 GyID 格式是否合法
    ///
    /// 返回 true 表示格式正确，false 表示格式错误。
    pub fn verify(gyid_str: &str) -> bool {
        use gyid_core::GyIdValidator;
        use gyid_core::identity::GyId;

        let gyid = GyId::new(gyid_str.to_string(), String::new(), 0);
        GyIdValidator::validate_format(gyid_str).unwrap_or(false)
            && gyid.is_valid_format()
    }

    // ───────────────────────────────────────────────────────────────────────────
    // ③ 设备关联
    // ───────────────────────────────────────────────────────────────────────────

    /// 执行设备关联操作
    ///
    /// - `opts.auth_code == "gen"` → 主设备模式：生成授权码
    /// - 其他 → 从设备模式：验证授权码并完成关联
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn link_device(opts: LinkOptions) -> SdkResult<serde_json::Value> {
        let storage = LocalStorage::new(None).map_err(SdkError::from)?;

        if opts.auth_code.eq_ignore_ascii_case("gen") {
            // 主设备：生成授权码
            let gyid_id = storage
                .get_config("last_gyid")
                .map_err(SdkError::from)?
                .ok_or_else(|| SdkError::InvalidParam("本机没有已保存的 GyID，请先生成".to_string()))?;

            let auth_code = generate_auth_code(&gyid_id);
            storage.save_config("pending_auth_code", &auth_code).map_err(SdkError::from)?;
            storage.save_config("pending_auth_master", &gyid_id).map_err(SdkError::from)?;

            let result = AuthCodeResult {
                auth_code: auth_code.clone(),
                master_gyid: gyid_id.clone(),
                expires_in_secs: 600,
                complete_command: format!("gyid link --auth-code {} --master {}", auth_code, gyid_id),
            };
            Ok(serde_json::to_value(result).map_err(SdkError::from)?)
        } else {
            // 从设备：验证授权码并完成关联
            let master_id = opts.master_gyid.as_deref().map(|s| s.to_string())
                .or_else(|| storage.get_config("pending_auth_master").ok().flatten())
                .ok_or_else(|| SdkError::InvalidParam("未指定主设备 GyID".to_string()))?;

            let now_secs = Utc::now().timestamp() as u64;
            if !verify_auth_code(&opts.auth_code, &master_id, now_secs) {
                return Err(SdkError::InvalidParam("授权码无效或已过期（10 分钟有效）".to_string()));
            }

            // 获取或生成本机 GyID
            let local_gyid = match storage.get_config("last_gyid").map_err(SdkError::from)? {
                Some(id) => storage.get_gyid(&id).map_err(SdkError::from)?
                    .ok_or_else(|| SdkError::Storage("本地 GyID 记录不完整".to_string()))?,
                None => {
                    let gyid = GyIdGenerator::new(GeneratorConfig::default())
                        .generate(None).await.map_err(SdkError::from)?;
                    storage.save_gyid(&gyid).map_err(SdkError::from)?;
                    storage.save_config("last_gyid", &gyid.id).map_err(SdkError::from)?;
                    gyid
                }
            };

            // 防重复关联
            let existing = storage.get_links(&master_id).map_err(SdkError::from)?;
            if existing.iter().any(|l| l.linked_id == local_gyid.id && l.active) {
                return Ok(serde_json::json!({
                    "message": "该设备已关联，无需重复操作",
                    "local_gyid": local_gyid.id,
                    "master_gyid": master_id
                }));
            }

            let now_ms = Utc::now().timestamp_millis() as u64;
            let link = DeviceLink {
                link_id: GyIdHasher::hash_strings(&[&master_id, &local_gyid.id]),
                master_id: master_id.clone(),
                linked_id: local_gyid.id.clone(),
                auth_code: opts.auth_code.clone(),
                linked_at: now_ms,
                active: true,
            };

            storage.save_link(&link).map_err(SdkError::from)?;

            if let Some(mut master_obj) = storage.get_gyid(&master_id).map_err(SdkError::from)? {
                master_obj.linked_devices += 1;
                let _ = storage.save_gyid(&master_obj);
            }

            let result = LinkResult {
                local_gyid: local_gyid.id,
                master_gyid: master_id,
                link_id: link.link_id,
                linked_at_ms: now_ms as i64,
                is_new_gyid: false,
            };
            Ok(serde_json::to_value(result).map_err(SdkError::from)?)
        }
    }

    // ───────────────────────────────────────────────────────────────────────────
    // ④ 设备列表
    // ───────────────────────────────────────────────────────────────────────────

    /// 获取关联设备列表
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_devices(gyid_str: Option<&str>) -> SdkResult<DeviceList> {
        let storage = LocalStorage::new(None).map_err(SdkError::from)?;

        let master_id = gyid_str
            .map(|s| s.to_string())
            .or_else(|| storage.get_config("last_gyid").ok().flatten())
            .ok_or_else(|| SdkError::InvalidParam("未找到已保存的 GyID".to_string()))?;

        let links = storage.get_links(&master_id).map_err(SdkError::from)?;

        let devices = links
            .into_iter()
            .map(|l| DeviceInfo {
                linked_gyid: l.linked_id,
                link_id: l.link_id,
                linked_at_ms: l.linked_at as i64,
                active: l.active,
            })
            .collect();

        Ok(DeviceList {
            master_gyid: master_id,
            devices,
        })
    }

    // ───────────────────────────────────────────────────────────────────────────
    // ⑤ 导出
    // ───────────────────────────────────────────────────────────────────────────

    /// 导出 GyID（JSON 字符串）
    #[cfg(not(target_arch = "wasm32"))]
    pub fn export_json() -> SdkResult<String> {
        let storage = LocalStorage::new(None).map_err(SdkError::from)?;
        let id = storage
            .get_config("last_gyid")
            .map_err(SdkError::from)?
            .ok_or_else(|| SdkError::InvalidParam("未找到已保存的 GyID".to_string()))?;

        let gyid = storage
            .get_gyid(&id)
            .map_err(SdkError::from)?
            .ok_or_else(|| SdkError::Storage("GyID 数据不完整".to_string()))?;

        let result = GyIdResult {
            id: gyid.id,
            hash: gyid.hash,
            created_at_ms: (gyid.created_at >> 16) as i64,
            linked_devices: gyid.linked_devices,
            platform: current_platform().to_string(),
            sdk_version: SDK_VERSION.to_string(),
        };

        serde_json::to_string_pretty(&result).map_err(SdkError::from)
    }

    // ───────────────────────────────────────────────────────────────────────────
    // ⑥ 版本信息
    // ───────────────────────────────────────────────────────────────────────────

    /// 获取 SDK 版本信息
    pub fn version() -> SdkVersion {
        let features = vec!["core".to_string()];

        #[cfg(target_arch = "wasm32")]
        let mut features = features;
        #[cfg(target_arch = "wasm32")]
        features.push("wasm".to_string());

        #[cfg(feature = "chain-signing")]
        features.push("chain-signing".to_string());

        SdkVersion {
            version: SDK_VERSION.to_string(),
            platform: current_platform().to_string(),
            features,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 内部辅助函数
// ─────────────────────────────────────────────────────────────────────────────

fn parse_geo_level(level: &str) -> SdkResult<GeoPrecisionLevel> {
    match level.to_lowercase().as_str() {
        "city" => Ok(GeoPrecisionLevel::City),
        "district" => Ok(GeoPrecisionLevel::District),
        "exact" => Ok(GeoPrecisionLevel::Exact),
        other => Err(SdkError::InvalidParam(format!(
            "无效的地理精度等级: '{}'，可选值: city/district/exact",
            other
        ))),
    }
}

fn parse_weights(w: &WeightsOptions) -> SdkResult<GyIdWeights> {
    let weights = GyIdWeights {
        hardware: w.hardware,
        geo: w.geo,
        timestamp: w.timestamp,
        avatar: w.avatar,
    };
    if !weights.is_valid() {
        return Err(SdkError::InvalidParam(format!(
            "权重之和必须为 1.0，当前为 {}",
            w.hardware + w.geo + w.timestamp + w.avatar
        )));
    }
    Ok(weights)
}

/// 生成授权码（供 SDK 内部调用）
fn generate_auth_code(master_id: &str) -> String {
    let prefix = if master_id.len() >= 8 {
        master_id[..8].to_uppercase()
    } else {
        format!("{:0<8}", master_id).to_uppercase()
    };
    let ts = Utc::now().timestamp() as u64;
    let ts_hex = format!("{:08X}", ts & 0xFFFF_FFFF);
    let hmac_input = format!("{}:{}", master_id, ts);
    let hmac = GyIdHasher::hash_strings(&[&hmac_input]);
    let hmac_short = hmac[..8].to_uppercase();
    format!("{}:{}:{}", prefix, ts_hex, hmac_short)
}

/// 验证授权码（供 SDK 内部调用）
fn verify_auth_code(code: &str, master_id: &str, now_secs: u64) -> bool {
    let parts: Vec<&str> = code.split(':').collect();
    if parts.len() != 3 {
        return false;
    }
    let ts = match u64::from_str_radix(parts[1], 16) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let elapsed = now_secs.saturating_sub(ts);
    if elapsed > 605 || ts > now_secs + 5 {
        return false;
    }
    let expected_prefix = if master_id.len() >= 8 {
        master_id[..8].to_uppercase()
    } else {
        format!("{:0<8}", master_id).to_uppercase()
    };
    if parts[0].to_uppercase() != expected_prefix {
        return false;
    }
    let hmac_input = format!("{}:{}", master_id, ts);
    let expected = GyIdHasher::hash_strings(&[&hmac_input]);
    parts[2].to_uppercase() == expected[..8].to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_gyid() {
        // 有效格式
        assert!(GyIdSdk::verify("GyID1234567890abcdefghijklmnopqr"));
        // 无效格式
        assert!(!GyIdSdk::verify("invalid"));
        assert!(!GyIdSdk::verify("XXXX1234567890abcdefghijklmnopqr"));
    }

    #[test]
    fn test_version() {
        let ver = GyIdSdk::version();
        assert!(!ver.version.is_empty());
        assert!(ver.features.contains(&"core".to_string()));
    }

    #[test]
    fn test_auth_code_roundtrip() {
        let master_id = "GyID_ABCDE12345_TEST";
        let code = generate_auth_code(master_id);
        let now = Utc::now().timestamp() as u64;
        assert!(verify_auth_code(&code, master_id, now));
    }

    #[test]
    fn test_parse_geo_level() {
        assert!(parse_geo_level("city").is_ok());
        assert!(parse_geo_level("district").is_ok());
        assert!(parse_geo_level("exact").is_ok());
        assert!(parse_geo_level("invalid").is_err());
    }
}
