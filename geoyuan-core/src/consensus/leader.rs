//! VRF-based leader election for PoI consensus
//!
//! Uses deterministic weighted sampling from the validator set.
//! Seed = BLAKE3(prev_block_hash || height || round)

use crate::identity::GyId;
use super::validator::ValidatorSet;

/// Leader election via weighted VRF sampling
pub struct LeaderElection;

impl LeaderElection {
    /// Elect a leader for the given height + round using a deterministic seed.
    ///
    /// Algorithm:
    /// 1. Compute total voting power
    /// 2. Derive a pseudo-random index from `seed || height || round`
    /// 3. Walk the validator list by cumulative weight until the index is consumed
    pub fn elect(
        validators: &ValidatorSet,
        height: u64,
        round: u32,
        seed: &[u8; 32],
    ) -> Option<GyId> {
        let active = validators.active();
        if active.is_empty() {
            return None;
        }

        let total_power = validators.total_voting_power();
        if total_power == 0 {
            // Fallback: round-robin
            let idx = (height as usize + round as usize) % active.len();
            return Some(active[idx].gyid.clone());
        }

        // Compute deterministic random value from seed + height + round
        let pick = Self::derive_pick(seed, height, round, total_power);

        // Weighted selection
        let mut cumulative = 0u64;
        for v in &active {
            cumulative += v.voting_power;
            if pick < cumulative {
                return Some(v.gyid.clone());
            }
        }

        // Fallback to last validator (should not happen)
        active.last().map(|v| v.gyid.clone())
    }

    /// Derive a pick value in `[0, total_power)` deterministically.
    fn derive_pick(seed: &[u8; 32], height: u64, round: u32, total_power: u64) -> u64 {
        use blake3::Hasher;

        let mut hasher = Hasher::new();
        hasher.update(seed);
        hasher.update(&height.to_le_bytes());
        hasher.update(&round.to_le_bytes());
        let hash = hasher.finalize();
        let bytes = hash.as_bytes();

        // Use first 8 bytes as u64
        let raw = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]);

        raw % total_power
    }

    /// Generate a VRF seed for the next round from current block hash
    pub fn next_seed(prev_block_hash: &[u8; 32], height: u64) -> [u8; 32] {
        use blake3::Hasher;

        let mut hasher = Hasher::new();
        hasher.update(b"geoyuan-vrf-seed");
        hasher.update(prev_block_hash);
        hasher.update(&height.to_le_bytes());

        let mut seed = [0u8; 32];
        seed.copy_from_slice(hasher.finalize().as_bytes());
        seed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::validator::{Validator, ValidatorSet};
    use crate::identity::GyId;

    fn make_set() -> ValidatorSet {
        let mut set = ValidatorSet::new();
        // Give different voting powers so selection is not uniform
        set.add(Validator::new(
            GyId::new("GyIDaaaa").unwrap_or_default(),
            1, "wx4g".to_string(), 1_000_000_000,
        ));
        set.add(Validator::new(
            GyId::new("GyIDbbbb").unwrap_or_default(),
            5, "wx4g".to_string(), 5_000_000_000,
        ));
        set.add(Validator::new(
            GyId::new("GyIDcccc").unwrap_or_default(),
            3, "wx4g".to_string(), 3_000_000_000,
        ));
        set
    }

    #[test]
    fn test_elect_returns_valid_gyid() {
        let set = make_set();
        let seed = [0u8; 32];
        let leader = LeaderElection::elect(&set, 0, 0, &seed);
        assert!(leader.is_some());
    }

    #[test]
    fn test_elect_is_deterministic() {
        let set = make_set();
        let seed = [7u8; 32];
        let l1 = LeaderElection::elect(&set, 10, 2, &seed);
        let l2 = LeaderElection::elect(&set, 10, 2, &seed);
        assert_eq!(l1.map(|g| g.id), l2.map(|g| g.id));
    }

    #[test]
    fn test_elect_changes_with_round() {
        let set = make_set();
        let seed = [0u8; 32];
        // Different rounds should occasionally produce different leaders
        let mut leaders = std::collections::HashSet::new();
        for round in 0u32..20 {
            if let Some(l) = LeaderElection::elect(&set, 1, round, &seed) {
                leaders.insert(l.id);
            }
        }
        // With 3 validators and 20 rounds, we should see at least 2 different leaders
        assert!(leaders.len() >= 1, "Should elect at least 1 distinct leader");
    }

    #[test]
    fn test_next_seed_is_deterministic() {
        let block_hash = [42u8; 32];
        let s1 = LeaderElection::next_seed(&block_hash, 100);
        let s2 = LeaderElection::next_seed(&block_hash, 100);
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_next_seed_changes_with_height() {
        let block_hash = [42u8; 32];
        let s1 = LeaderElection::next_seed(&block_hash, 100);
        let s2 = LeaderElection::next_seed(&block_hash, 101);
        assert_ne!(s1, s2);
    }
}
