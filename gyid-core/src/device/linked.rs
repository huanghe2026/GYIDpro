//! 从设备模块

use crate::{identity::GyId, device::{Device, DeviceType, DeviceLink}};

/// 从设备
pub struct LinkedDevice {
    device: Device,
    auth_code: String,
    master_id: String,
}

impl LinkedDevice {
    /// 创建新的从设备
    pub fn new(gyid: &GyId, auth_code: &str, master_id: &str) -> Self {
        let device = Device::new(
            gyid.id.clone(),
            DeviceType::Linked,
            gyid.id.clone(),
        );
        Self {
            device,
            auth_code: auth_code.to_string(),
            master_id: master_id.to_string(),
        }
    }
    
    /// 获取设备信息
    pub fn device(&self) -> &Device {
        &self.device
    }
    
    /// 获取关联信息
    pub fn link_info(&self) -> (&str, &str) {
        (&self.auth_code, &self.master_id)
    }
    
    /// 验证授权码
    pub fn verify_auth_code(&self, code: &str) -> bool {
        self.auth_code == code
    }
    
    /// 从设备关联创建
    pub fn from_link(gyid: &GyId, link: &DeviceLink) -> Self {
        Self::new(gyid, &link.auth_code, &link.master_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GyId;

    #[test]
    fn test_linked_device() {
        let gyid = GyId::new(
            "GyID_linked1234567890abcdefghijk".to_string(),
            "def456".to_string(),
            1711814400000,
        );
        
        let linked = LinkedDevice::new(
            &gyid,
            "auth_code_123",
            "master_id_456",
        );
        
        println!("Linked device: {:?}", linked.device());
        assert!(linked.verify_auth_code("auth_code_123"));
        assert!(!linked.verify_auth_code("wrong_code"));
    }
}
