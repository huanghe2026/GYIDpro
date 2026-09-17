//! Geoyuan - Non-fungible token representing a photo at a location

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};


/// How this Geoyuan was minted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MintType {
    /// Minted from a GPS photo (default, +1 GY)
    #[default]
    Photo,
    /// Minted from a continuous GPS trajectory (+0.1~0.5 GY)
    Trajectory,
    /// First minting in an H3 cell — genesis reward (+2 GY)
    GeoDiscovery,
    /// PoL node online reward (+0.5 GY/day)
    PolNode,
    /// Dual-person verification at the same H3 cell (+0.5 GY each)
    DualVerification,
    /// Consecutive 7-day check-in chain in the same city (+1 GY)
    CheckinChain,
}

impl MintType {
    /// Default reward value in milli-GY (1000 = 1 GY)
    pub fn default_reward_milli_gy(&self) -> u64 {
        match self {
            MintType::Photo => 1000,
            MintType::Trajectory => 100, // overridden by distance
            MintType::GeoDiscovery => 2000,
            MintType::PolNode => 500,
            MintType::DualVerification => 500,
            MintType::CheckinChain => 1000,
        }
    }
}

impl std::fmt::Display for MintType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MintType::Photo => write!(f, "Photo"),
            MintType::Trajectory => write!(f, "Trajectory"),
            MintType::GeoDiscovery => write!(f, "GeoDiscovery"),
            MintType::PolNode => write!(f, "PolNode"),
            MintType::DualVerification => write!(f, "DualVerification"),
            MintType::CheckinChain => write!(f, "CheckinChain"),
        }
    }
}

/// Geoyuan - One Geoyuan per photo with GPS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Geoyuan {
    /// Unique coin ID (UUID)
    pub id: String,

    /// Owner GyID
    pub owner: String,

    /// GPS latitude where photo was taken
    pub latitude: f64,

    /// GPS longitude where photo was taken
    pub longitude: f64,

    /// Geohash for the location
    pub geohash: String,

    /// SHA256 hash of the photo
    pub photo_hash: String,

    /// When this coin was minted
    pub minted_at: DateTime<Utc>,

    /// Coin status
    pub status: CoinStatus,

    /// Chain transaction hash (if minted on-chain)
    pub chain_tx: Option<String>,

    /// Previous transaction hash (for transfer history)
    pub previous_tx: Option<String>,

    /// How this coin was minted
    #[serde(default)]
    pub mint_type: MintType,

    /// Coin value in milli-GY (1000 = 1 GY)
    #[serde(default = "default_photo_value")]
    pub value_milli_gy: u64,
}

fn default_photo_value() -> u64 {
    1000
}

impl Geoyuan {
    /// Create a new Geoyuan
    pub fn new(
        id: String,
        owner: String,
        latitude: f64,
        longitude: f64,
    ) -> Self {
        let geohash = crate::geo::geohash::encode(latitude, longitude, 9);

        Self {
            id,
            owner,
            latitude,
            longitude,
            geohash,
            photo_hash: String::new(), // Set by minter
            minted_at: Utc::now(),
            status: CoinStatus::Minted,
            chain_tx: None,
            previous_tx: None,
            mint_type: MintType::Photo,
            value_milli_gy: MintType::Photo.default_reward_milli_gy(),
        }
    }

    /// Create with photo hash
    pub fn with_photo_hash(mut self, photo_hash: String) -> Self {
        self.photo_hash = photo_hash;
        self
    }

    /// Create with chain transaction
    pub fn with_chain_tx(mut self, tx: String) -> Self {
        self.chain_tx = Some(tx);
        self
    }

    /// Set mint type and recalculate value
    pub fn with_mint_type(mut self, mint_type: MintType) -> Self {
        self.mint_type = mint_type;
        self.value_milli_gy = mint_type.default_reward_milli_gy();
        self
    }

    /// Set explicit value in milli-GY
    pub fn with_value_milli_gy(mut self, value: u64) -> Self {
        self.value_milli_gy = value;
        self
    }

    /// Get coin display name (truncated)
    pub fn display_name(&self) -> String {
        format!("Geoyuan#{}", &self.id[..8])
    }

    /// Calculate distance to another coin
    pub fn distance_to(&self, other: &Geoyuan) -> f64 {
        let coord1 = crate::photo::GpsCoordinates::new(self.latitude, self.longitude);
        let coord2 = crate::photo::GpsCoordinates::new(other.latitude, other.longitude);
        coord1.distance_to(&coord2)
    }

    /// Value in GY (float)
    pub fn value_gy(&self) -> f64 {
        self.value_milli_gy as f64 / 1000.0
    }
}

/// GeoCoin status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CoinStatus {
    /// Coin has been minted, owned by someone
    #[default]
    Minted,
    
    /// Coin is being transferred
    Transferring,
    
    /// Coin has been burned
    Burned,
}

impl std::fmt::Display for CoinStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoinStatus::Minted => write!(f, "Minted"),
            CoinStatus::Transferring => write!(f, "Transferring"),
            CoinStatus::Burned => write!(f, "Burned"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    
    #[test]
    fn test_create_coin() {
        let coin = Geoyuan::new(
            Uuid::new_v4().to_string(),
            "GeoID7xK9m2AbCdEfGh123456".to_string(),
            39.9042,
            116.4074,
        );
        
        assert!(!coin.geohash.is_empty());
        assert_eq!(coin.status, CoinStatus::Minted);
        assert!(coin.chain_tx.is_none());
    }
    
    #[test]
    fn test_coin_display_name() {
        let coin = Geoyuan::new(
            "12345678-1234-1234-1234-123456789abc".to_string(),
            "GeoID7xK9m2AbCdEfGh123456".to_string(),
            39.9042,
            116.4074,
        );
        
        assert!(coin.display_name().starts_with("Geoyuan#"));
    }
    
    #[test]
    fn test_coin_distance() {
        let coin1 = Geoyuan::new(
            Uuid::new_v4().to_string(),
            "GeoID7xK9m2AbCdEfGh123456".to_string(),
            39.9042,
            116.4074,
        );
        
        let coin2 = Geoyuan::new(
            Uuid::new_v4().to_string(),
            "GeoID7xK9m2AbCdEfGh123456".to_string(),
            39.9073,
            116.3972,
        );
        
        let distance = coin1.distance_to(&coin2);
        assert!(distance > 100.0);
    }
}
