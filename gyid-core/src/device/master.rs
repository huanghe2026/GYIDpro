//! 主设备模块

use crate::{GyIdError, Result, identity::GyId, device::{Device, DeviceType, DeviceLink}};
use crate::crypto::GyIdHasher;

/// 主设备
pub struct MasterDevice {
    device: Device,
}

impl MasterDevice {
    /// 创建新的主设备
    pub fn new(gyid: &GyId) -> Self {
        let device = Device::new(
            gyid.id.clone(),
            DeviceType::Master,
            gyid.id.clone(),
        );
        Self { device }
    }
    
    /// 获取设备信息
    pub fn device(&self) -> &Device {
        &self.device
    }
    
    /// 生成授权码
    pub fn generate_auth_code(&self) -> String {
        let data = format!("{}:{}", self.device.device_id, chrono::Utc::now().timestamp());
        GyIdHasher::hash_strings(&[&data])
    }
    
    /// 创建设备关联
    pub fn create_link(&self, linked_device_id: &str) -> Result<DeviceLink> {
        if self.device.gyid == linked_device_id {
            return Err(GyIdError::DeviceLinkError(
                "不能关联到主设备本身".to_string()
            ));
        }
        
        Ok(DeviceLink {
            link_id: GyIdHasher::hash_strings(&[
                &self.device.device_id,
                linked_device_id,
            ]),
            master_id: self.device.device_id.clone(),
            linked_id: linked_device_id.to_string(),
            auth_code: self.generate_auth_code(),
            linked_at: chrono::Utc::now().timestamp_millis() as u64,
            active: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GyId;

    #[test]
    fn test_master_device() {
        let gyid = GyId::new(
            "GyID_test1234567890abcdefghijkl".to_string(),
            "abc123".to_string(),
            1711814400000,
        );
        
        let master = MasterDevice::new(&gyid);
        println!("Master: {:?}", master.device());
        
        let auth_code = master.generate_auth_code();
        println!("Auth code: {}", auth_code);
    }
}
