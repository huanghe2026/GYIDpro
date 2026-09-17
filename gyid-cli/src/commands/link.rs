//! link 命令 - 设备关联
//!
//! 流程分两种角色：
//!
//! **主设备**（已有 GyID，想授权新设备）：
//!   1. `gyid link --gen-code`  → 生成 16 位授权码并显示
//!   2. 将授权码告知新设备用户
//!
//! **从设备**（新设备，想关联主 GyID）：
//!   1. `gyid link --auth-code <CODE> [--master <MASTER_GYID>]`
//!   2. 验证授权码 → 生成本机 GyID → 存储关联 → 打印结果
//!
//! 授权码格式（本地离线验证）：
//!   `<master_id_prefix_8>:<timestamp_hex_8>:<hmac_8>`
//!   总长 27 字符，全大写 Base58 字符集。

use gyid_core::{
    GyIdGenerator, GeneratorConfig,
    device::DeviceLink,
    storage::LocalStorage,
    crypto::GyIdHasher,
    identity::GyId,
};
use chrono::Utc;

// ─────────────────────────────────────────────────────────────────────────────
// 公开入口
// ─────────────────────────────────────────────────────────────────────────────

/// CLI 入口：根据参数决定扮演主设备还是从设备角色
pub async fn run(auth_code: &str, master_gyid: Option<&str>) -> anyhow::Result<()> {
    let storage = LocalStorage::new(None)?;

    // 如果 auth_code 是特殊关键字 "gen"，则生成授权码（主设备模式）
    if auth_code.eq_ignore_ascii_case("gen") {
        return run_generate_code(&storage, master_gyid).await;
    }

    // 否则：从设备模式，用授权码完成关联
    run_link_device(&storage, auth_code, master_gyid).await
}

// ─────────────────────────────────────────────────────────────────────────────
// 主设备模式：生成授权码
// ─────────────────────────────────────────────────────────────────────────────

async fn run_generate_code(
    storage: &LocalStorage,
    explicit_gyid: Option<&str>,
) -> anyhow::Result<()> {
    println!("🔑 主设备：生成授权码\n");

    // 找到本设备的 GyID
    let gyid = resolve_gyid(storage, explicit_gyid)?;

    let code = generate_auth_code(&gyid.id);

    // 存储"待关联"授权码到 config 表，key = pending_auth_code
    storage.save_config("pending_auth_code", &code)?;
    storage.save_config("pending_auth_master", &gyid.id)?;

    println!("✅ 授权码生成成功！\n");
    println!("   主设备 GyID : {}", gyid.id);
    println!("   授权码      : {}", code);
    println!("   有效期      : 10 分钟\n");
    println!("📱 请在新设备上运行以下命令完成关联：");
    println!(
        "   gyid link --auth-code {} --master {}\n",
        code, gyid.id
    );
    println!("⏳ 等待新设备完成关联后，可用 `gyid devices` 查看关联列表。");

    // 此处实际生产可开启一个短暂的本地 TCP server 等待从设备回调
    // 本期实现离线授权码模式，不需要网络连接

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// 从设备模式：验证授权码并完成关联
// ─────────────────────────────────────────────────────────────────────────────

async fn run_link_device(
    storage: &LocalStorage,
    auth_code: &str,
    master_gyid_str: Option<&str>,
) -> anyhow::Result<()> {
    println!("🔗 从设备：验证授权码并关联\n");

    // 1. 解析授权码
    let parsed = parse_auth_code(auth_code)
        .ok_or_else(|| anyhow::anyhow!("授权码格式不正确，请检查是否完整复制"))?;

    // 2. 确定主设备 GyID
    let master_id = if let Some(id) = master_gyid_str {
        id.to_string()
    } else {
        // 用授权码中的前缀猜测
        storage
            .get_config("pending_auth_master")?
            .ok_or_else(|| anyhow::anyhow!("未找到主设备 GyID，请通过 --master 参数指定"))?
    };

    // 3. 验证授权码（时间戳有效性 + HMAC）
    let now_secs = Utc::now().timestamp() as u64;
    if !verify_auth_code(auth_code, &master_id, now_secs) {
        return Err(anyhow::anyhow!(
            "授权码验证失败：可能已过期（10 分钟有效）或主设备 GyID 不匹配"
        ));
    }

    println!("✅ 授权码验证通过");
    println!("   主设备 ID : {}", master_id);
    println!("   时间戳    : {} ({}s 前生成)\n", parsed.timestamp, now_secs - parsed.timestamp);

    // 4. 检查是否已有本机 GyID，若没有则生成
    let local_gyid = match storage.get_config("last_gyid")? {
        Some(id) => {
            if let Some(g) = storage.get_gyid(&id)? {
                println!("📋 使用本机已有 GyID: {}", g.id);
                g
            } else {
                generate_local_gyid(storage).await?
            }
        }
        None => generate_local_gyid(storage).await?,
    };

    // 5. 防止重复关联
    let existing_links = storage.get_links(&master_id)?;
    if existing_links
        .iter()
        .any(|l| l.linked_id == local_gyid.id && l.active)
    {
        println!("ℹ️  该设备已关联到主设备 {}，无需重复操作。", master_id);
        return Ok(());
    }

    // 6. 使用事务保存设备关联记录
    let link = DeviceLink {
        link_id: GyIdHasher::hash_strings(&[&master_id, &local_gyid.id]),
        master_id: master_id.clone(),
        linked_id: local_gyid.id.clone(),
        auth_code: auth_code.to_string(),
        linked_at: Utc::now().timestamp_millis() as u64,
        active: true,
    };

    // 事务操作：保存关联 + 更新主设备计数
    let master_id_for_update = master_id.clone();
    storage.with_transaction(|_tx| {
        // 保存关联
        storage.save_link(&link)?;
        
        // 更新主设备的关联计数（如果本地也有主设备 GyID 记录）
        if let Some(mut master_gyid_obj) = storage.get_gyid(&master_id_for_update)? {
            master_gyid_obj.linked_devices += 1;
            storage.save_gyid(&master_gyid_obj)?;
        }

        Ok(())
    })?;

    // 清除临时授权码
    let _ = storage.save_config("pending_auth_code", "");
    let _ = storage.save_config("pending_auth_master", "");

    println!("🎉 设备关联成功！\n");
    println!("   本机 GyID : {}", local_gyid.id);
    println!("   主设备 ID : {}", master_id);
    println!("   关联 ID   : {}", link.link_id);
    println!("   关联时间  : {}\n", chrono::DateTime::from_timestamp_millis(link.linked_at as i64)
        .map(|t: chrono::DateTime<Utc>| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "未知".to_string()));
    println!("✨ 使用 `gyid devices` 查看所有关联设备");

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// 授权码生成与验证逻辑
// ─────────────────────────────────────────────────────────────────────────────

/// 授权码结构（解析后）
struct ParsedAuthCode {
    /// 主设备 ID 前 8 字符
    master_prefix: String,
    /// 生成时间戳（秒）
    timestamp: u64,
    /// HMAC 校验段
    hmac: String,
}

/// 生成授权码
///
/// 格式：`<master_prefix_8>:<ts_hex_8>:<hmac_8>`
/// 示例：`ABCDEFGH:61F4A2B3:3C9D2F1A`
fn generate_auth_code(master_id: &str) -> String {
    let prefix = if master_id.len() >= 8 {
        master_id[..8].to_string()
    } else {
        format!("{:0<8}", master_id)
    }
    .to_uppercase();

    let ts = Utc::now().timestamp() as u64;
    let ts_hex = format!("{:08X}", ts & 0xFFFF_FFFF); // 低 32 位

    // HMAC = 前 8 字符的 BLAKE3 哈希
    let hmac_input = format!("{}:{}", master_id, ts);
    let hmac = GyIdHasher::hash_strings(&[&hmac_input]);
    let hmac_short = hmac[..8].to_uppercase();

    format!("{}:{}:{}", prefix, ts_hex, hmac_short)
}

/// 解析授权码字符串
fn parse_auth_code(code: &str) -> Option<ParsedAuthCode> {
    let parts: Vec<&str> = code.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let master_prefix = parts[0].to_string();
    let ts = u64::from_str_radix(parts[1], 16).ok()?;
    let hmac = parts[2].to_string();

    Some(ParsedAuthCode {
        master_prefix,
        timestamp: ts,
        hmac,
    })
}

/// 验证授权码是否有效
///
/// 检查：
/// 1. 格式正确
/// 2. 时间戳在 10 分钟内（前后容差 5 秒）
/// 3. HMAC 与主设备 ID + 时间戳一致
fn verify_auth_code(code: &str, master_id: &str, now_secs: u64) -> bool {
    let parsed = match parse_auth_code(code) {
        Some(p) => p,
        None => return false,
    };

    // 时间戳有效性（10 分钟 = 600 秒，允许 5 秒时钟偏差）
    let elapsed = now_secs.saturating_sub(parsed.timestamp);
    if elapsed > 605 {
        return false;
    }
    // 防止未来时间戳（超过 5 秒说明时钟不同步）
    if parsed.timestamp > now_secs + 5 {
        return false;
    }

    // 校验主设备前缀
    let expected_prefix = if master_id.len() >= 8 {
        master_id[..8].to_uppercase()
    } else {
        format!("{:0<8}", master_id).to_uppercase()
    };
    if parsed.master_prefix.to_uppercase() != expected_prefix {
        return false;
    }

    // 重新计算 HMAC 并比较
    let hmac_input = format!("{}:{}", master_id, parsed.timestamp);
    let expected_hmac = GyIdHasher::hash_strings(&[&hmac_input]);
    let expected_short = expected_hmac[..8].to_uppercase();

    parsed.hmac.to_uppercase() == expected_short
}

// ─────────────────────────────────────────────────────────────────────────────
// 辅助函数
// ─────────────────────────────────────────────────────────────────────────────

/// 从存储中找到已保存的 GyID，或通过参数指定
fn resolve_gyid(storage: &LocalStorage, explicit: Option<&str>) -> anyhow::Result<GyId> {
    let id = if let Some(id) = explicit {
        id.to_string()
    } else {
        storage
            .get_config("last_gyid")?
            .ok_or_else(|| anyhow::anyhow!("本机没有已保存的 GyID，请先运行 `gyid generate` 生成"))?
    };

    storage
        .get_gyid(&id)?
        .ok_or_else(|| anyhow::anyhow!("找不到 GyID \"{}\"，请先运行 `gyid generate`", id))
}

/// 在本机生成新的 GyID 并保存到存储
async fn generate_local_gyid(storage: &LocalStorage) -> anyhow::Result<GyId> {
    println!("⚙️  本机尚无 GyID，正在自动生成...\n");
    let config = GeneratorConfig::default();
    let generator = GyIdGenerator::new(config);
    let gyid = generator.generate(None).await
        .map_err(|e| anyhow::anyhow!("GyID 生成失败: {}", e))?;

    storage.save_gyid(&gyid)?;
    storage.save_config("last_gyid", &gyid.id)?;

    println!("✅ 本机 GyID 已生成: {}\n", gyid.id);
    Ok(gyid)
}

// ─────────────────────────────────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_code_roundtrip() {
        let master_id = "GyID_ABCDE12345_FGHIJ67890_KLMNO";
        let code = generate_auth_code(master_id);
        println!("生成的授权码: {}", code);

        let now = Utc::now().timestamp() as u64;
        assert!(
            verify_auth_code(&code, master_id, now),
            "刚生成的授权码应该验证通过"
        );
    }

    #[test]
    fn test_expired_auth_code() {
        let master_id = "GyID_ABCDE12345_FGHIJ67890_KLMNO";
        let code = generate_auth_code(master_id);

        // 模拟 11 分钟后
        let future_now = Utc::now().timestamp() as u64 + 660;
        assert!(
            !verify_auth_code(&code, master_id, future_now),
            "过期授权码应该验证失败"
        );
    }

    #[test]
    fn test_wrong_master_auth_code() {
        let master_id = "GyID_ABCDE12345_FGHIJ67890_KLMNO";
        let code = generate_auth_code(master_id);

        let now = Utc::now().timestamp() as u64;
        assert!(
            !verify_auth_code(&code, "GyID_ZZZZZ99999_WRONG_MASTER_ID", now),
            "错误 master_id 应验证失败"
        );
    }

    #[test]
    fn test_parse_invalid_code() {
        assert!(parse_auth_code("INVALID").is_none());
        assert!(parse_auth_code("A:B").is_none());
        assert!(parse_auth_code("A:GGGGGGGG:C").is_none()); // 非十六进制时间戳
    }
}
