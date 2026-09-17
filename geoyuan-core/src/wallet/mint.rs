//! Coin minter - mints new Geoyuans from photos

use super::{Geoyuan, Wallet};
use crate::{Result, GeoYuanError, identity::GyId};
use uuid::Uuid;

/// Coin minter - creates new Geoyuans from photos
pub struct CoinMinter;

impl CoinMinter {
    /// Mint a new Geoyuan from a photo and GyID
    pub fn mint(gy_id: &GyId, photo_hash: &str) -> Result<Geoyuan> {
        // Require GPS coordinates
        if gy_id.latitude == 0.0 && gy_id.longitude == 0.0 {
            return Err(GeoYuanError::Wallet("Invalid coordinates".to_string()));
        }
        
        // Generate unique coin ID
        let coin_id = Uuid::new_v4().to_string();
        
        // Create coin
        let coin = Geoyuan::new(
            coin_id,
            gy_id.id.clone(),
            gy_id.latitude,
            gy_id.longitude,
        )
        .with_photo_hash(photo_hash.to_string());
        
        Ok(coin)
    }
    
    /// Mint and add to wallet
    pub fn mint_to_wallet(
        gy_id: &GyId,
        photo_hash: &str,
        wallet: &mut Wallet,
    ) -> Result<Geoyuan> {
        let coin = Self::mint(gy_id, photo_hash)?;
        wallet.add_coin(coin.clone());
        Ok(coin)
    }
    
    /// Batch mint multiple coins (one per photo)
    pub fn batch_mint(
        gy_ids: &[GyId],
        photo_hashes: &[String],
        wallet: &mut Wallet,
    ) -> Result<Vec<Geoyuan>> {
        if gy_ids.len() != photo_hashes.len() {
            return Err(GeoYuanError::Wallet(
                "GyIDs and photo hashes count mismatch".to_string()
            ));
        }
        
        let mut coins = Vec::new();
        
        for (gy_id, photo_hash) in gy_ids.iter().zip(photo_hashes.iter()) {
            let coin = Self::mint_to_wallet(gy_id, photo_hash, wallet)?;
            coins.push(coin);
        }
        
        Ok(coins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::GyId;
    use chrono::Utc;
    
    fn create_test_gy_id() -> GyId {
        GyId {
            id: "GyID7xK9m2AbCdEfGh123456".to_string(),
            hash: "0".repeat(64),
            created_at: Utc::now(),
            photo_hash: "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".to_string(),
            latitude: 39.9042,
            longitude: 116.4074,
            geohash: "wx4g0e6md5".to_string(),
            version: 1,
        }
    }
    
    #[test]
    fn test_mint_coin() {
        let gy_id = create_test_gy_id();
        let photo_hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        
        let coin = CoinMinter::mint(&gy_id, photo_hash).unwrap();
        
        assert_eq!(coin.owner, gy_id.id);
        assert_eq!(coin.latitude, gy_id.latitude);
        assert_eq!(coin.longitude, gy_id.longitude);
        assert_eq!(coin.photo_hash, photo_hash);
    }
    
    #[test]
    fn test_mint_to_wallet() {
        let gy_id = create_test_gy_id();
        let photo_hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        
        let mut wallet = Wallet::new(
            gy_id.id.clone(),
            "0".repeat(64),
        );
        
        let coin = CoinMinter::mint_to_wallet(&gy_id, photo_hash, &mut wallet).unwrap();
        
        assert_eq!(wallet.balance, 1);
        assert!(wallet.get_coin(&coin.id).is_some());
    }
    
    #[test]
    fn test_batch_mint() {
        let gy_ids = vec![
            create_test_gy_id(),
            {
                let mut gy_id = create_test_gy_id();
                gy_id.id = "GyID8yL0n3BcDeFgHi789012".to_string();
                gy_id.latitude = 40.0;
                gy_id.longitude = 117.0;
                gy_id
            },
        ];
        
        let photo_hashes = vec![
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".to_string(),
            "a87ff679a2f3e71d9181a67b7542122c4e4ecee65360cc180d0d0f6740dd4ce3".to_string(),
        ];
        
        let mut wallet = Wallet::new(
            gy_ids[0].id.clone(),
            "0".repeat(64),
        );
        
        let coins = CoinMinter::batch_mint(&gy_ids, &photo_hashes, &mut wallet).unwrap();
        
        assert_eq!(coins.len(), 2);
        assert_eq!(wallet.balance, 2);
    }
}
