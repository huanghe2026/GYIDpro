//! PoI (Proof of Identity) Consensus Module
//!
//! GeoYuan's consensus mechanism is based on verified identities.
//! Nodes are elected as block producers proportional to their GyID count,
//! geo-diversity, and stake, using a VRF-based leader selection.
//!
//! # Core Concepts
//!
//! - **Validator**: A node with at least 1 verified GyID
//! - **Voting Power**: Derived from GyID count + geo-diversity score + stake
//! - **Leader Selection**: VRF (Verifiable Random Function) with weighted sampling
//! - **Block Time**: Target 6 seconds
//! - **Federation**: Each geo-region forms a sub-network (federation)
//! - **Finality**: BFT-style 2/3+ voting for block finalization

pub mod leader;
pub mod validator;
pub mod vote;

pub use leader::LeaderElection;
pub use validator::{Validator, ValidatorSet};
pub use vote::{Vote, VoteType, ConsensusRound};

use serde::{Deserialize, Serialize};
use crate::identity::GyId;

/// Consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Target block time in milliseconds
    pub block_time_ms: u64,
    /// Minimum validators for BFT consensus
    pub min_validators: usize,
    /// BFT quorum threshold (2/3+)
    pub quorum_threshold_pct: u8,
    /// VRF seed rotation interval (blocks)
    pub vrf_seed_rotation: u64,
    /// Federation size (max validators per geo-region)
    pub federation_size: usize,
    /// Stake required to become a validator (nanoGY)
    pub min_stake: u64,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            block_time_ms: 6000,
            min_validators: 4,
            quorum_threshold_pct: 67, // 2/3+ of validators
            vrf_seed_rotation: 100,
            federation_size: 21,
            min_stake: 1_000_000_000, // 1 GY in nanoGY
        }
    }
}

/// Consensus state machine phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConsensusPhase {
    /// Waiting for the round to start
    #[default]
    Idle,
    /// Leader is proposing a block
    Propose,
    /// Validators are pre-voting
    PreVote,
    /// Validators are pre-committing
    PreCommit,
    /// Block is committed to the chain
    Commit,
}

impl std::fmt::Display for ConsensusPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusPhase::Idle => write!(f, "Idle"),
            ConsensusPhase::Propose => write!(f, "Propose"),
            ConsensusPhase::PreVote => write!(f, "PreVote"),
            ConsensusPhase::PreCommit => write!(f, "PreCommit"),
            ConsensusPhase::Commit => write!(f, "Commit"),
        }
    }
}

/// Consensus engine state
#[derive(Debug, Default)]
pub struct ConsensusEngine {
    /// Current block height
    pub height: u64,
    /// Current round number within a height
    pub round: u32,
    /// Current phase
    pub phase: ConsensusPhase,
    /// Configuration
    pub config: ConsensusConfig,
    /// Validator set
    pub validators: ValidatorSet,
    /// Current leader GyID
    pub current_leader: Option<GyId>,
    /// Active consensus rounds
    pub active_rounds: Vec<ConsensusRound>,
}

impl ConsensusEngine {
    /// Create new consensus engine
    pub fn new(config: ConsensusConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Advance to next block height
    pub fn advance_height(&mut self) {
        self.height += 1;
        self.round = 0;
        self.phase = ConsensusPhase::Idle;
        self.current_leader = None;
        // Clear rounds for old height
        self.active_rounds.retain(|r| r.height >= self.height);
    }

    /// Start a new consensus round
    pub fn start_round(&mut self, seed: &[u8; 32]) -> Option<GyId> {
        self.phase = ConsensusPhase::Propose;

        // Elect leader using VRF-based weighted sampling
        let leader = LeaderElection::elect(&self.validators, self.height, self.round, seed);
        self.current_leader = leader.clone();

        let round = ConsensusRound::new(self.height, self.round);
        self.active_rounds.push(round);

        leader
    }

    /// Get the current round
    pub fn current_round(&self) -> Option<&ConsensusRound> {
        self.active_rounds
            .iter()
            .rev()
            .find(|r| r.height == self.height && r.round == self.round)
    }

    /// Get the current round (mutable)
    pub fn current_round_mut(&mut self) -> Option<&mut ConsensusRound> {
        let height = self.height;
        let round = self.round;
        self.active_rounds
            .iter_mut()
            .rev()
            .find(|r| r.height == height && r.round == round)
    }

    /// Check if we have enough votes to proceed
    pub fn check_quorum(&self, vote_count: usize) -> bool {
        let total = self.validators.len();
        if total == 0 { return false; }
        let required = (total * self.config.quorum_threshold_pct as usize).div_ceil(100);
        vote_count >= required
    }

    /// Add a validator
    pub fn add_validator(&mut self, validator: Validator) {
        self.validators.add(validator);
    }

    /// Remove a validator
    pub fn remove_validator(&mut self, gyid: &GyId) {
        self.validators.remove(gyid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::GyId;

    fn make_validator(id: &str, gyid_count: u32, stake: u64) -> Validator {
        Validator::new(
            GyId::new(id).unwrap_or_default(),
            gyid_count,
            "wx4g0e6md5".to_string(), // Beijing geohash
            stake,
        )
    }

    #[test]
    fn test_consensus_engine_basic() {
        let mut engine = ConsensusEngine::new(ConsensusConfig::default());

        // Add validators
        engine.add_validator(make_validator("GyIDaaaa", 3, 5_000_000_000));
        engine.add_validator(make_validator("GyIDbbbb", 2, 3_000_000_000));
        engine.add_validator(make_validator("GyIDcccc", 5, 8_000_000_000));

        assert_eq!(engine.validators.len(), 3);

        // Start round with a seed
        let seed = [42u8; 32];
        let leader = engine.start_round(&seed);
        assert!(leader.is_some(), "Should elect a leader");
        assert_eq!(engine.phase, ConsensusPhase::Propose);
    }

    #[test]
    fn test_quorum_check() {
        let mut engine = ConsensusEngine::new(ConsensusConfig::default());

        for i in 0..10 {
            engine.add_validator(make_validator(
                &format!("GyIDtest{:04}", i),
                1,
                1_000_000_000,
            ));
        }

        // 10 validators, 67% threshold = ceil(6.7) = 7
        assert!(!engine.check_quorum(6));
        assert!(engine.check_quorum(7));
        assert!(engine.check_quorum(10));
    }

    #[test]
    fn test_advance_height() {
        let mut engine = ConsensusEngine::new(ConsensusConfig::default());
        engine.add_validator(make_validator("GyIDaaaa", 1, 1_000_000_000));

        engine.start_round(&[0u8; 32]);
        assert_eq!(engine.height, 0);
        assert!(engine.current_leader.is_some());

        engine.advance_height();
        assert_eq!(engine.height, 1);
        assert_eq!(engine.round, 0);
        assert!(engine.current_leader.is_none());
    }
}
