//! Validator definitions for PoI consensus

use serde::{Deserialize, Serialize};
use crate::identity::GyId;

/// A consensus validator node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    /// Validator's primary GyID
    pub gyid: GyId,

    /// Number of verified GyIDs this validator holds
    pub gyid_count: u32,

    /// Geohash of validator's primary location
    pub geohash: String,

    /// Staked amount (nanoGY)
    pub stake: u64,

    /// Voting power (computed from gyid_count + geo_diversity + stake)
    pub voting_power: u64,

    /// Is this validator active
    pub active: bool,

    /// Last seen timestamp (Unix ms)
    pub last_seen: i64,
}

impl Validator {
    /// Create a new validator and compute initial voting power
    pub fn new(gyid: GyId, gyid_count: u32, geohash: String, stake: u64) -> Self {
        let vp = Self::compute_voting_power(gyid_count, stake);
        Self {
            gyid,
            gyid_count,
            geohash,
            stake,
            voting_power: vp,
            active: true,
            last_seen: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Compute voting power:
    ///   voting_power = gyid_count * 1_000_000
    ///                + stake / 1_000_000   (nanoGY → GY weight)
    ///
    /// This balances identity richness with economic stake.
    pub fn compute_voting_power(gyid_count: u32, stake: u64) -> u64 {
        let identity_weight = gyid_count as u64 * 1_000_000;
        let stake_weight = stake / 1_000_000; // every 1 GY = 1 stake point
        identity_weight + stake_weight
    }

    /// Update voting power (call after changing gyid_count or stake)
    pub fn refresh_voting_power(&mut self) {
        self.voting_power = Self::compute_voting_power(self.gyid_count, self.stake);
    }

    /// Mark this validator as seen now
    pub fn touch(&mut self) {
        self.last_seen = chrono::Utc::now().timestamp_millis();
    }
}

/// Ordered set of validators
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidatorSet {
    validators: Vec<Validator>,
}

impl ValidatorSet {
    /// Create empty set
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or update a validator
    pub fn add(&mut self, v: Validator) {
        if let Some(existing) = self.validators.iter_mut().find(|x| x.gyid == v.gyid) {
            *existing = v;
        } else {
            self.validators.push(v);
        }
    }

    /// Remove validator by GyID
    pub fn remove(&mut self, gyid: &GyId) {
        self.validators.retain(|v| &v.gyid != gyid);
    }

    /// Get validator by GyID
    pub fn get(&self, gyid: &GyId) -> Option<&Validator> {
        self.validators.iter().find(|v| &v.gyid == gyid)
    }

    /// Get all active validators
    pub fn active(&self) -> Vec<&Validator> {
        self.validators.iter().filter(|v| v.active).collect()
    }

    /// Total number of validators (including inactive)
    pub fn len(&self) -> usize {
        self.validators.len()
    }

    /// Number of active validators
    pub fn active_count(&self) -> usize {
        self.validators.iter().filter(|v| v.active).count()
    }

    /// Total voting power across all active validators
    pub fn total_voting_power(&self) -> u64 {
        self.validators
            .iter()
            .filter(|v| v.active)
            .map(|v| v.voting_power)
            .sum()
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.validators.is_empty()
    }

    /// All validators (slice)
    pub fn all(&self) -> &[Validator] {
        &self.validators
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::GyId;

    fn gyid(s: &str) -> GyId {
        GyId::new(s).unwrap_or_default()
    }

    #[test]
    fn test_voting_power_calculation() {
        // 3 GyIDs + 5 GY stake = 3*1M + 5 = 3_000_005
        let vp = Validator::compute_voting_power(3, 5_000_000_000);
        assert_eq!(vp, 3 * 1_000_000 + 5000);
    }

    #[test]
    fn test_validator_set_add_remove() {
        let mut set = ValidatorSet::new();

        let v1 = Validator::new(gyid("GyIDaaaa"), 2, "wx4g".to_string(), 1_000_000_000);
        let v2 = Validator::new(gyid("GyIDbbbb"), 1, "wx4g".to_string(), 2_000_000_000);

        set.add(v1.clone());
        set.add(v2);
        assert_eq!(set.len(), 2);
        assert_eq!(set.active_count(), 2);

        // Update v1
        let updated = Validator::new(gyid("GyIDaaaa"), 5, "wx4g".to_string(), 1_000_000_000);
        set.add(updated);
        assert_eq!(set.len(), 2); // Still 2, just updated

        let found = set.get(&gyid("GyIDaaaa")).unwrap();
        assert_eq!(found.gyid_count, 5);

        // Remove
        set.remove(&gyid("GyIDaaaa"));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_total_voting_power() {
        let mut set = ValidatorSet::new();
        set.add(Validator::new(gyid("GyIDaaaa"), 3, "wx4g".to_string(), 0));
        set.add(Validator::new(gyid("GyIDbbbb"), 2, "wx4g".to_string(), 0));

        // 3M + 2M = 5M
        assert_eq!(set.total_voting_power(), 5_000_000);
    }
}
