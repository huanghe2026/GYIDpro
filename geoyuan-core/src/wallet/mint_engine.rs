//! Multi-method minting engine
//!
//! Beyond the basic photo-mint (+1 GY), this module implements:
//! - TrackMinter: continuous GPS trajectory (+0.1~0.5 GY)
//! - DiscoveryMinter: first minting in an H3 cell (+2 GY)
//! - PolNodeMinter: PoL node online reward (+0.5 GY/day)
//! - DualVerificationMinter: same-cell dual-person mint (+0.5 GY each)
//! - CheckinChainMinter: 7-day consecutive check-in in same city (+1 GY)

use super::{Geoyuan, MintType, Wallet};
use crate::identity::GyId;
use crate::photo::GpsCoordinates;
use crate::{GeoYuanError, Result};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

// ────────────────────────────────────────────────────────────────────────────
// TrackMinter — 轨迹铸币
// ────────────────────────────────────────────────────────────────────────────

/// A single point in a GPS trajectory
#[derive(Debug, Clone)]
pub struct TrackPoint {
    /// GPS coordinates
    pub lat: f64,
    pub lon: f64,
    /// Timestamp of this point
    pub timestamp: DateTime<Utc>,
}

impl TrackPoint {
    /// Create a new track point
    pub fn new(lat: f64, lon: f64, timestamp: DateTime<Utc>) -> Self {
        Self {
            lat,
            lon,
            timestamp,
        }
    }

    fn distance_to(&self, other: &TrackPoint) -> f64 {
        GpsCoordinates::new(self.lat, self.lon)
            .distance_to(&GpsCoordinates::new(other.lat, other.lon))
    }
}

/// Activity type for trajectory minting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackActivity {
    /// Walking — minimum 1 km
    Walk,
    /// Cycling — minimum 3 km
    Bike,
}

impl TrackActivity {
    /// Minimum distance in meters required for this activity
    pub fn min_distance_m(&self) -> f64 {
        match self {
            TrackActivity::Walk => 1000.0,
            TrackActivity::Bike => 3000.0,
        }
    }
}

/// Trajectory minter — rewards continuous GPS movement
pub struct TrackMinter;

impl TrackMinter {
    /// Calculate total distance of a trajectory (meters)
    pub fn total_distance(points: &[TrackPoint]) -> f64 {
        if points.len() < 2 {
            return 0.0;
        }
        points.windows(2).map(|w| w[0].distance_to(&w[1])).sum()
    }

    /// Calculate total duration in seconds
    pub fn total_duration_secs(points: &[TrackPoint]) -> i64 {
        if points.len() < 2 {
            return 0;
        }
        (points[points.len() - 1].timestamp - points[0].timestamp).num_seconds()
    }

    /// Compute reward in milli-GY based on distance.
    /// 0.1 GY per km, capped at 0.5 GY (500 milli-GY).
    pub fn reward_milli_gy(distance_m: f64) -> u64 {
        let km = distance_m / 1000.0;
        let reward = (km * 100.0).round() as u64;
        reward.min(500).max(0)
    }

    /// Mint a trajectory coin.
    ///
    /// Validation:
    /// - At least 2 track points
    /// - Total distance ≥ activity minimum
    /// - Duration > 0
    pub fn mint(gy_id: &GyId, points: &[TrackPoint], activity: TrackActivity) -> Result<Geoyuan> {
        if points.len() < 2 {
            return Err(GeoYuanError::Wallet(
                "Trajectory requires at least 2 points".to_string(),
            ));
        }

        let distance = Self::total_distance(points);
        let min_dist = activity.min_distance_m();

        if distance < min_dist {
            return Err(GeoYuanError::Wallet(format!(
                "Trajectory distance {:.0}m below minimum {:.0}m for {:?}",
                distance, min_dist, activity
            )));
        }

        let duration = Self::total_duration_secs(points);
        if duration <= 0 {
            return Err(GeoYuanError::Wallet(
                "Trajectory duration must be positive".to_string(),
            ));
        }

        // Use the midpoint of the trajectory as the coin's location
        let mid_idx = points.len() / 2;
        let mid = &points[mid_idx];

        let reward = Self::reward_milli_gy(distance);

        // Build a deterministic "photo hash" from the trajectory
        let track_hash = format!(
            "track:{}:{}:{}:{}",
            gy_id.id,
            points[0].timestamp.timestamp(),
            points[points.len() - 1].timestamp.timestamp(),
            distance as u64
        );

        let coin = Geoyuan::new(
            Uuid::new_v4().to_string(),
            gy_id.id.clone(),
            mid.lat,
            mid.lon,
        )
        .with_photo_hash(track_hash)
        .with_mint_type(MintType::Trajectory)
        .with_value_milli_gy(reward);

        Ok(coin)
    }

    /// Mint and add to wallet
    pub fn mint_to_wallet(
        gy_id: &GyId,
        points: &[TrackPoint],
        activity: TrackActivity,
        wallet: &mut Wallet,
    ) -> Result<Geoyuan> {
        let coin = Self::mint(gy_id, points, activity)?;
        wallet.add_coin(coin.clone());
        Ok(coin)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// DiscoveryMinter — 地理发现奖励
// ────────────────────────────────────────────────────────────────────────────

/// Discovery minter — rewards the first minting in an H3 cell.
///
/// Caller supplies the set of already-minted H3 cell IDs. If the target
/// cell is not in that set, a genesis reward coin (+2 GY) is minted.
pub struct DiscoveryMinter;

/// H3 resolution used for discovery detection.
/// Res 12 ≈ 9m edge — fine-grained enough to reward genuine first discoveries.
pub const DISCOVERY_H3_RESOLUTION: u8 = 12;

impl DiscoveryMinter {
    /// Check whether the given H3 cell has already been minted.
    pub fn is_first_discovery(h3_cell: &str, minted_cells: &[String]) -> bool {
        !minted_cells.iter().any(|c| c == h3_cell)
    }

    /// Compute the H3 cell for a lat/lon at discovery resolution.
    pub fn h3_cell_for(lat: f64, lon: f64) -> Option<String> {
        let cell =
            crate::geo::h3grid::gps_to_cell_with_resolution(lat, lon, DISCOVERY_H3_RESOLUTION)?;
        Some(crate::geo::h3grid::cell_to_string(cell))
    }

    /// Mint a geo-discovery genesis coin.
    ///
    /// Validation:
    /// - GyID must have valid coordinates
    /// - H3 cell must not be in `minted_cells`
    pub fn mint(gy_id: &GyId, photo_hash: &str, minted_cells: &[String]) -> Result<Geoyuan> {
        if gy_id.latitude == 0.0 && gy_id.longitude == 0.0 {
            return Err(GeoYuanError::Wallet("Invalid coordinates".to_string()));
        }

        let h3_cell = Self::h3_cell_for(gy_id.latitude, gy_id.longitude)
            .ok_or_else(|| GeoYuanError::Wallet("Failed to compute H3 cell".to_string()))?;

        if !Self::is_first_discovery(&h3_cell, minted_cells) {
            return Err(GeoYuanError::Wallet(format!(
                "H3 cell {} already discovered",
                h3_cell
            )));
        }

        let coin = Geoyuan::new(
            Uuid::new_v4().to_string(),
            gy_id.id.clone(),
            gy_id.latitude,
            gy_id.longitude,
        )
        .with_photo_hash(photo_hash.to_string())
        .with_mint_type(MintType::GeoDiscovery);

        Ok(coin)
    }

    /// Mint and add to wallet
    pub fn mint_to_wallet(
        gy_id: &GyId,
        photo_hash: &str,
        minted_cells: &[String],
        wallet: &mut Wallet,
    ) -> Result<Geoyuan> {
        let coin = Self::mint(gy_id, photo_hash, minted_cells)?;
        wallet.add_coin(coin.clone());
        Ok(coin)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// PolNodeMinter — PoL 节点奖励
// ────────────────────────────────────────────────────────────────────────────

/// PoL node minter — rewards nodes that stay online with stable GPS.
///
/// Validation:
/// - Online hours ≥ 8
/// - GPS must be stable (flag provided by caller)
/// - At most one reward per day (caller enforces by checking last reward date)
pub struct PolNodeMinter;

/// Minimum online hours per day to qualify
pub const POL_MIN_ONLINE_HOURS: u64 = 8;

/// H3 resolution for PoL stability check (Res 9 ≈ 174m edge)
pub const POL_H3_RESOLUTION: u8 = 9;

impl PolNodeMinter {
    /// Mint a PoL node reward coin.
    pub fn mint(gy_id: &GyId, online_hours: u64, gps_stable: bool) -> Result<Geoyuan> {
        if online_hours < POL_MIN_ONLINE_HOURS {
            return Err(GeoYuanError::Wallet(format!(
                "Online hours {} below minimum {}",
                online_hours, POL_MIN_ONLINE_HOURS
            )));
        }

        if !gps_stable {
            return Err(GeoYuanError::Wallet(
                "GPS not stable during online period".to_string(),
            ));
        }

        if gy_id.latitude == 0.0 && gy_id.longitude == 0.0 {
            return Err(GeoYuanError::Wallet("Invalid coordinates".to_string()));
        }

        // Build a deterministic hash for the daily reward
        let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
        let pol_hash = format!("pol:{}:{}:{}", gy_id.id, today, online_hours);

        let coin = Geoyuan::new(
            Uuid::new_v4().to_string(),
            gy_id.id.clone(),
            gy_id.latitude,
            gy_id.longitude,
        )
        .with_photo_hash(pol_hash)
        .with_mint_type(MintType::PolNode);

        Ok(coin)
    }

    /// Mint and add to wallet
    pub fn mint_to_wallet(
        gy_id: &GyId,
        online_hours: u64,
        gps_stable: bool,
        wallet: &mut Wallet,
    ) -> Result<Geoyuan> {
        let coin = Self::mint(gy_id, online_hours, gps_stable)?;
        wallet.add_coin(coin.clone());
        Ok(coin)
    }

    /// Check whether a wallet has already received a PoL reward today.
    pub fn has_rewarded_today(wallet: &Wallet) -> bool {
        let today = Utc::now().date_naive();
        wallet
            .coins
            .iter()
            .filter(|c| c.mint_type == MintType::PolNode)
            .any(|c| c.minted_at.date_naive() == today)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// DualVerificationMinter — 双人验证
// ────────────────────────────────────────────────────────────────────────────

/// Dual-verification minter — rewards two users minting in the same H3 cell
/// at nearly the same time.
///
/// Validation:
/// - Two distinct GyIDs
/// - Both in the same H3 cell (Res 12)
/// - Timestamps within `MAX_TIME_DELTA_SECS` of each other
pub struct DualVerificationMinter;

/// Maximum allowed time difference between the two minters (seconds)
pub const MAX_TIME_DELTA_SECS: i64 = 300; // 5 minutes

/// H3 resolution for dual verification (same as discovery)
pub const DUAL_H3_RESOLUTION: u8 = DISCOVERY_H3_RESOLUTION;

impl DualVerificationMinter {
    /// Verify that two GyIDs qualify for dual verification.
    pub fn verify_pair(
        gy_id_a: &GyId,
        ts_a: DateTime<Utc>,
        gy_id_b: &GyId,
        ts_b: DateTime<Utc>,
    ) -> Result<()> {
        if gy_id_a.id == gy_id_b.id {
            return Err(GeoYuanError::Wallet(
                "Dual verification requires two distinct users".to_string(),
            ));
        }

        let cell_a = DiscoveryMinter::h3_cell_for(gy_id_a.latitude, gy_id_a.longitude)
            .ok_or_else(|| GeoYuanError::Wallet("Failed to compute H3 for user A".to_string()))?;
        let cell_b = DiscoveryMinter::h3_cell_for(gy_id_b.latitude, gy_id_b.longitude)
            .ok_or_else(|| GeoYuanError::Wallet("Failed to compute H3 for user B".to_string()))?;

        if cell_a != cell_b {
            return Err(GeoYuanError::Wallet(format!(
                "Users are in different H3 cells: {} vs {}",
                cell_a, cell_b
            )));
        }

        let delta = (ts_a - ts_b).num_seconds().abs();
        if delta > MAX_TIME_DELTA_SECS {
            return Err(GeoYuanError::Wallet(format!(
                "Time delta {}s exceeds max {}s",
                delta, MAX_TIME_DELTA_SECS
            )));
        }

        Ok(())
    }

    /// Mint a dual-verification coin for user A (caller mints for B separately).
    pub fn mint(gy_id: &GyId, partner_gy_id: &GyId, timestamp: DateTime<Utc>) -> Result<Geoyuan> {
        // Self-verification not allowed
        if gy_id.id == partner_gy_id.id {
            return Err(GeoYuanError::Wallet(
                "Cannot dual-verify with self".to_string(),
            ));
        }

        let dv_hash = format!(
            "dual:{}:{}:{}",
            gy_id.id,
            partner_gy_id.id,
            timestamp.timestamp()
        );

        let coin = Geoyuan::new(
            Uuid::new_v4().to_string(),
            gy_id.id.clone(),
            gy_id.latitude,
            gy_id.longitude,
        )
        .with_photo_hash(dv_hash)
        .with_mint_type(MintType::DualVerification);

        Ok(coin)
    }

    /// Mint a pair of dual-verification coins (one for each user) and add to
    /// respective wallets.
    pub fn mint_pair(
        gy_id_a: &GyId,
        wallet_a: &mut Wallet,
        gy_id_b: &GyId,
        wallet_b: &mut Wallet,
        timestamp: DateTime<Utc>,
    ) -> Result<(Geoyuan, Geoyuan)> {
        Self::verify_pair(gy_id_a, timestamp, gy_id_b, timestamp)?;

        let coin_a = Self::mint(gy_id_a, gy_id_b, timestamp)?;
        let coin_b = Self::mint(gy_id_b, gy_id_a, timestamp)?;

        wallet_a.add_coin(coin_a.clone());
        wallet_b.add_coin(coin_b.clone());

        Ok((coin_a, coin_b))
    }
}

// ────────────────────────────────────────────────────────────────────────────
// CheckinChainMinter — 签到链
// ────────────────────────────────────────────────────────────────────────────

/// A single check-in record
#[derive(Debug, Clone)]
pub struct CheckinRecord {
    /// Date of check-in (date only, time ignored for chain purposes)
    pub date: chrono::NaiveDate,
    /// City identifier (e.g. geohash prefix or city code)
    pub city: String,
}

/// Check-in chain minter — rewards 7 consecutive days in the same city.
///
/// Validation:
/// - At least 7 check-in records
/// - Consecutive dates (no gaps)
/// - All in the same city
pub struct CheckinChainMinter;

/// Required consecutive days
pub const REQUIRED_CHECKIN_DAYS: usize = 7;

impl CheckinChainMinter {
    /// Validate a check-in chain.
    ///
    /// Returns the number of consecutive days if valid, or an error.
    pub fn validate_chain(records: &[CheckinRecord]) -> Result<usize> {
        if records.len() < REQUIRED_CHECKIN_DAYS {
            return Err(GeoYuanError::Wallet(format!(
                "Need at least {} check-in records, got {}",
                REQUIRED_CHECKIN_DAYS,
                records.len()
            )));
        }

        // Sort by date
        let mut sorted: Vec<&CheckinRecord> = records.iter().collect();
        sorted.sort_by_key(|r| r.date);

        // Find the longest consecutive same-city run
        let mut best_run = 1usize;
        let mut current_run = 1usize;

        for window in sorted.windows(2) {
            let prev = &window[0];
            let curr = &window[1];

            let day_diff = (curr.date - prev.date).num_days();
            if day_diff == 1 && curr.city == prev.city {
                current_run += 1;
                best_run = best_run.max(current_run);
            } else {
                current_run = 1;
            }
        }

        if best_run < REQUIRED_CHECKIN_DAYS {
            return Err(GeoYuanError::Wallet(format!(
                "Longest consecutive same-city chain is {} days, need {}",
                best_run, REQUIRED_CHECKIN_DAYS
            )));
        }

        Ok(best_run)
    }

    /// Mint a check-in chain reward coin.
    pub fn mint(gy_id: &GyId, records: &[CheckinRecord]) -> Result<Geoyuan> {
        Self::validate_chain(records)?;

        if gy_id.latitude == 0.0 && gy_id.longitude == 0.0 {
            return Err(GeoYuanError::Wallet("Invalid coordinates".to_string()));
        }

        // Use the last check-in city for the coin
        let last_city = records
            .iter()
            .max_by_key(|r| r.date)
            .map(|r| r.city.clone())
            .unwrap_or_default();

        let chain_hash = format!("checkin:{}:{}:{}", gy_id.id, last_city, records.len());

        let coin = Geoyuan::new(
            Uuid::new_v4().to_string(),
            gy_id.id.clone(),
            gy_id.latitude,
            gy_id.longitude,
        )
        .with_photo_hash(chain_hash)
        .with_mint_type(MintType::CheckinChain);

        Ok(coin)
    }

    /// Mint and add to wallet
    pub fn mint_to_wallet(
        gy_id: &GyId,
        records: &[CheckinRecord],
        wallet: &mut Wallet,
    ) -> Result<Geoyuan> {
        let coin = Self::mint(gy_id, records)?;
        wallet.add_coin(coin.clone());
        Ok(coin)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::GyId;

    fn make_gy_id(lat: f64, lon: f64) -> GyId {
        GyId {
            id: format!("GyID{}{}", lat.abs() as u32, lon.abs() as u32),
            hash: "0".repeat(64),
            created_at: Utc::now(),
            photo_hash: "test_photo_hash".to_string(),
            latitude: lat,
            longitude: lon,
            geohash: crate::geo::geohash::encode(lat, lon, 9),
            version: 1,
        }
    }

    fn make_gy_id_named(name: &str, lat: f64, lon: f64) -> GyId {
        GyId {
            id: name.to_string(),
            hash: "0".repeat(64),
            created_at: Utc::now(),
            photo_hash: "test_photo_hash".to_string(),
            latitude: lat,
            longitude: lon,
            geohash: crate::geo::geohash::encode(lat, lon, 9),
            version: 1,
        }
    }

    // ── TrackMinter ──────────────────────────────────────────────────────────

    #[test]
    fn test_track_mint_walk_valid() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let base = Utc::now();
        // ~1.5 km walk (lat delta ~0.0138 ≈ 1.53km)
        let points = vec![
            TrackPoint::new(39.8930, 116.4000, base),
            TrackPoint::new(39.9068, 116.4074, base + Duration::minutes(20)),
        ];

        let coin = TrackMinter::mint(&gy_id, &points, TrackActivity::Walk).unwrap();
        assert_eq!(coin.mint_type, MintType::Trajectory);
        assert!(coin.value_milli_gy >= 100 && coin.value_milli_gy <= 500);
    }

    #[test]
    fn test_track_mint_walk_too_short() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let base = Utc::now();
        let points = vec![
            TrackPoint::new(39.9042, 116.4074, base),
            TrackPoint::new(39.9043, 116.4075, base + Duration::minutes(1)),
        ];

        let err = TrackMinter::mint(&gy_id, &points, TrackActivity::Walk).unwrap_err();
        assert!(err.to_string().contains("below minimum"));
    }

    #[test]
    fn test_track_mint_bike_valid() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let base = Utc::now();
        // ~3.5 km bike ride
        let points = vec![
            TrackPoint::new(39.8800, 116.4000, base),
            TrackPoint::new(39.9100, 116.4200, base + Duration::minutes(15)),
        ];

        let coin = TrackMinter::mint(&gy_id, &points, TrackActivity::Bike).unwrap();
        assert_eq!(coin.mint_type, MintType::Trajectory);
        assert!(coin.value_milli_gy >= 100);
    }

    #[test]
    fn test_track_reward_capped() {
        // Very long distance should cap at 500 milli-GY (0.5 GY)
        let reward = TrackMinter::reward_milli_gy(100_000.0); // 100 km
        assert_eq!(reward, 500);
    }

    #[test]
    fn test_track_reward_zero() {
        let reward = TrackMinter::reward_milli_gy(0.0);
        assert_eq!(reward, 0);
    }

    #[test]
    fn test_track_mint_to_wallet() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let base = Utc::now();
        let points = vec![
            TrackPoint::new(39.8930, 116.4000, base),
            TrackPoint::new(39.9068, 116.4074, base + Duration::minutes(20)),
        ];
        let mut wallet = Wallet::new(gy_id.id.clone(), "0".repeat(64));

        TrackMinter::mint_to_wallet(&gy_id, &points, TrackActivity::Walk, &mut wallet).unwrap();
        assert_eq!(wallet.balance, 1);
        assert!(wallet.total_value_milli_gy() > 0);
    }

    // ── DiscoveryMinter ──────────────────────────────────────────────────────

    #[test]
    fn test_discovery_first() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let minted_cells = vec![];

        let coin = DiscoveryMinter::mint(&gy_id, "photo_hash", &minted_cells).unwrap();
        assert_eq!(coin.mint_type, MintType::GeoDiscovery);
        assert_eq!(coin.value_milli_gy, 2000); // 2 GY
    }

    #[test]
    fn test_discovery_already_minted() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let cell = DiscoveryMinter::h3_cell_for(gy_id.latitude, gy_id.longitude).unwrap();
        let minted_cells = vec![cell];

        let err = DiscoveryMinter::mint(&gy_id, "photo_hash", &minted_cells).unwrap_err();
        assert!(err.to_string().contains("already discovered"));
    }

    #[test]
    fn test_discovery_different_cell_ok() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        // A cell far away
        let minted_cells = vec!["872830828ffffff".to_string()];

        let coin = DiscoveryMinter::mint(&gy_id, "photo_hash", &minted_cells);
        assert!(coin.is_ok());
    }

    // ── PolNodeMinter ────────────────────────────────────────────────────────

    #[test]
    fn test_pol_node_valid() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let coin = PolNodeMinter::mint(&gy_id, 10, true).unwrap();
        assert_eq!(coin.mint_type, MintType::PolNode);
        assert_eq!(coin.value_milli_gy, 500);
    }

    #[test]
    fn test_pol_node_insufficient_hours() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let err = PolNodeMinter::mint(&gy_id, 5, true).unwrap_err();
        assert!(err.to_string().contains("below minimum"));
    }

    #[test]
    fn test_pol_node_gps_unstable() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let err = PolNodeMinter::mint(&gy_id, 10, false).unwrap_err();
        assert!(err.to_string().contains("not stable"));
    }

    #[test]
    fn test_pol_node_rewarded_today() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let mut wallet = Wallet::new(gy_id.id.clone(), "0".repeat(64));
        assert!(!PolNodeMinter::has_rewarded_today(&wallet));

        PolNodeMinter::mint_to_wallet(&gy_id, 10, true, &mut wallet).unwrap();
        assert!(PolNodeMinter::has_rewarded_today(&wallet));
    }

    // ── DualVerificationMinter ───────────────────────────────────────────────

    #[test]
    fn test_dual_verification_pair_valid() {
        let gy_id_a = make_gy_id_named("GyID_A", 39.9042, 116.4074);
        let gy_id_b = make_gy_id_named("GyID_B", 39.9042, 116.4074); // same spot
        let ts = Utc::now();

        DualVerificationMinter::verify_pair(&gy_id_a, ts, &gy_id_b, ts).unwrap();
    }

    #[test]
    fn test_dual_verification_same_user_fails() {
        let gy_id = make_gy_id_named("GyID_X", 39.9042, 116.4074);
        let ts = Utc::now();

        let err = DualVerificationMinter::verify_pair(&gy_id, ts, &gy_id, ts).unwrap_err();
        assert!(err.to_string().contains("distinct users"));
    }

    #[test]
    fn test_dual_verification_different_cell_fails() {
        let gy_id_a = make_gy_id_named("GyID_A", 39.9042, 116.4074);
        let gy_id_b = make_gy_id_named("GyID_B", 40.0000, 117.0000); // far away
        let ts = Utc::now();

        let err = DualVerificationMinter::verify_pair(&gy_id_a, ts, &gy_id_b, ts).unwrap_err();
        assert!(err.to_string().contains("different H3 cells"));
    }

    #[test]
    fn test_dual_verification_time_delta_fails() {
        let gy_id_a = make_gy_id_named("GyID_A", 39.9042, 116.4074);
        let gy_id_b = make_gy_id_named("GyID_B", 39.9042, 116.4074);
        let ts_a = Utc::now();
        let ts_b = ts_a + Duration::minutes(10); // > 5 min

        let err = DualVerificationMinter::verify_pair(&gy_id_a, ts_a, &gy_id_b, ts_b).unwrap_err();
        assert!(err.to_string().contains("Time delta"));
    }

    #[test]
    fn test_dual_verification_mint_pair() {
        let gy_id_a = make_gy_id_named("GyID_A", 39.9042, 116.4074);
        let gy_id_b = make_gy_id_named("GyID_B", 39.9042, 116.4074);
        let ts = Utc::now();

        let mut wallet_a = Wallet::new(gy_id_a.id.clone(), "0".repeat(64));
        let mut wallet_b = Wallet::new(gy_id_b.id.clone(), "1".repeat(64));

        let (coin_a, coin_b) =
            DualVerificationMinter::mint_pair(&gy_id_a, &mut wallet_a, &gy_id_b, &mut wallet_b, ts)
                .unwrap();

        assert_eq!(coin_a.mint_type, MintType::DualVerification);
        assert_eq!(coin_b.mint_type, MintType::DualVerification);
        assert_eq!(wallet_a.balance, 1);
        assert_eq!(wallet_b.balance, 1);
    }

    // ── CheckinChainMinter ───────────────────────────────────────────────────

    fn make_checkin_chain(days: usize, city: &str) -> Vec<CheckinRecord> {
        let base = chrono::NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        (0..days)
            .map(|i| CheckinRecord {
                date: base + chrono::Duration::days(i as i64),
                city: city.to_string(),
            })
            .collect()
    }

    #[test]
    fn test_checkin_chain_valid() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let records = make_checkin_chain(7, "beijing");

        let coin = CheckinChainMinter::mint(&gy_id, &records).unwrap();
        assert_eq!(coin.mint_type, MintType::CheckinChain);
        assert_eq!(coin.value_milli_gy, 1000);
    }

    #[test]
    fn test_checkin_chain_too_short() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let records = make_checkin_chain(5, "beijing");

        let err = CheckinChainMinter::mint(&gy_id, &records).unwrap_err();
        assert!(err.to_string().contains("at least 7"));
    }

    #[test]
    fn test_checkin_chain_gap_fails() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let mut records = make_checkin_chain(8, "beijing");
        // Create a gap by moving day 3 to day 10 (8 records, but chain broken)
        records[3].date = chrono::NaiveDate::from_ymd_opt(2026, 7, 11).unwrap();

        let err = CheckinChainMinter::mint(&gy_id, &records).unwrap_err();
        assert!(err.to_string().contains("Longest consecutive"));
    }

    #[test]
    fn test_checkin_chain_different_city_fails() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let mut records = make_checkin_chain(7, "beijing");
        // Change city on day 4
        records[3].city = "shanghai".to_string();

        let err = CheckinChainMinter::mint(&gy_id, &records).unwrap_err();
        assert!(err.to_string().contains("Longest consecutive"));
    }

    #[test]
    fn test_checkin_chain_to_wallet() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let records = make_checkin_chain(7, "beijing");
        let mut wallet = Wallet::new(gy_id.id.clone(), "0".repeat(64));

        CheckinChainMinter::mint_to_wallet(&gy_id, &records, &mut wallet).unwrap();
        assert_eq!(wallet.balance, 1);
        assert_eq!(wallet.total_value_gy(), 1.0);
    }

    // ── Wallet integration ───────────────────────────────────────────────────

    #[test]
    fn test_wallet_multi_mint_total_value() {
        let gy_id = make_gy_id(39.9042, 116.4074);
        let mut wallet = Wallet::new(gy_id.id.clone(), "0".repeat(64));

        // Photo mint (1 GY)
        let photo_coin = Geoyuan::new(
            Uuid::new_v4().to_string(),
            gy_id.id.clone(),
            39.9042,
            116.4074,
        )
        .with_photo_hash("photo1".to_string());
        wallet.add_coin(photo_coin);

        // PoL node reward (0.5 GY)
        PolNodeMinter::mint_to_wallet(&gy_id, 10, true, &mut wallet).unwrap();

        // Check-in chain (1 GY)
        let records = make_checkin_chain(7, "beijing");
        CheckinChainMinter::mint_to_wallet(&gy_id, &records, &mut wallet).unwrap();

        assert_eq!(wallet.balance, 3);
        // 1000 + 500 + 1000 = 2500 milli-GY = 2.5 GY
        assert_eq!(wallet.total_value_milli_gy(), 2500);
        assert!((wallet.total_value_gy() - 2.5).abs() < 0.001);

        assert_eq!(wallet.count_by_type(MintType::Photo), 1);
        assert_eq!(wallet.count_by_type(MintType::PolNode), 1);
        assert_eq!(wallet.count_by_type(MintType::CheckinChain), 1);
    }
}
