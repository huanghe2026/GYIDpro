//! GyID validator
//! 
//! Validates GyID format and authenticity

use super::{GyId, GeoIdConfig};

/// GyID Validator
pub struct GyIdValidator {
    config: GeoIdConfig,
}

impl GyIdValidator {
    /// Create new validator
    pub fn new(config: GeoIdConfig) -> Self {
        Self { config }
    }
    
    /// Create with default config
    pub fn new_default() -> Self {
        Self::new(GeoIdConfig::default())
    }
    
    /// Validate GyID format
    pub fn validate_format(&self, id: &str) -> ValidationResult {
        // Check prefix
        let expected_prefix = if self.config.testnet { "TGyID" } else { "GyID" };
        
        if !id.starts_with(expected_prefix) {
            return ValidationResult::Invalid(format!(
                "Invalid prefix: expected '{}', got '{}'",
                expected_prefix,
                &id[..id.len().min(5)]
            ));
        }
        
        // Check length (GyID + 20-22 chars)
        let min_len = expected_prefix.len() + 20;
        let max_len = expected_prefix.len() + 22;
        
        if id.len() < min_len || id.len() > max_len {
            return ValidationResult::Invalid(format!(
                "Invalid length: expected {}-{} chars, got {}",
                min_len, max_len, id.len()
            ));
        }
        
        // Check characters (Base58 alphabet)
        let chars = &id[expected_prefix.len()..];
        for c in chars.chars() {
            if !c.is_ascii_alphanumeric() {
                return ValidationResult::Invalid(format!(
                    "Invalid character in GyID: '{}'",
                    c
                ));
            }
        }
        
        ValidationResult::Valid
    }
    
    /// Validate full GyId struct
    pub fn validate(&self, gy_id: &GyId) -> ValidationResult {
        // Validate format
        let format_result = self.validate_format(&gy_id.id);
        if !matches!(format_result, ValidationResult::Valid) {
            return format_result;
        }
        
        // Check version
        if gy_id.version != self.config.version {
            return ValidationResult::Invalid(format!(
                "Version mismatch: expected {}, got {}",
                self.config.version, gy_id.version
            ));
        }
        
        // Check coordinates range
        if gy_id.latitude < -90.0 || gy_id.latitude > 90.0 {
            return ValidationResult::Invalid("Invalid latitude range".to_string());
        }
        
        if gy_id.longitude < -180.0 || gy_id.longitude > 180.0 {
            return ValidationResult::Invalid("Invalid longitude range".to_string());
        }
        
        // Check hash format (64 hex chars)
        if gy_id.hash.len() != 64 || !gy_id.hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return ValidationResult::Invalid("Invalid hash format".to_string());
        }
        
        // Check photo hash format (64 hex chars)
        if gy_id.photo_hash.len() != 64 || !gy_id.photo_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return ValidationResult::Invalid("Invalid photo hash format".to_string());
        }
        
        // Check geohash is not empty
        if gy_id.geohash.is_empty() {
            return ValidationResult::Invalid("Empty geohash".to_string());
        }
        
        // Check timestamp is valid
        if gy_id.created_at.timestamp() <= 0 {
            return ValidationResult::Invalid("Invalid timestamp".to_string());
        }
        
        ValidationResult::Valid
    }
    
    /// Quick check if string looks like a valid GyID
    pub fn is_gy_id(&self, s: &str) -> bool {
        matches!(self.validate_format(s), ValidationResult::Valid)
    }
}

/// Validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Valid GyID
    Valid,
    /// Invalid with reason
    Invalid(String),
}

impl ValidationResult {
    /// Check if valid
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }
    
    /// Get error message if invalid
    pub fn error(&self) -> Option<&str> {
        match self {
            ValidationResult::Valid => None,
            ValidationResult::Invalid(msg) => Some(msg),
        }
    }
}

impl std::fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationResult::Valid => write!(f, "Valid"),
            ValidationResult::Invalid(msg) => write!(f, "Invalid: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_format() {
        let validator = GyIdValidator::new_default();
        
        // Valid
        assert!(validator.validate_format("GyID7xK9m2AbCdEfGh123456").is_valid());
        
        // Invalid prefix
        assert!(!validator.validate_format("TGyID7xK9m2AbCdEfGh123456").is_valid());
        
        // Invalid length
        assert!(!validator.validate_format("GyIDshort").is_valid());
        assert!(!validator.validate_format("GyIDthis_is_much_too_long_to_be_valid").is_valid());
        
        // Invalid characters
        assert!(!validator.validate_format("GyID7xK9m2AbCd!@#$%").is_valid());
    }
    
    #[test]
    fn test_is_gy_id() {
        let validator = GyIdValidator::new_default();
        
        assert!(validator.is_gy_id("GyID7xK9m2AbCdEfGh123456"));
        assert!(!validator.is_gy_id("NotGyID7xK9m2AbCdEfGh123456"));
        assert!(!validator.is_gy_id("just_some_text"));
        assert!(!validator.is_gy_id(""));
    }
}
