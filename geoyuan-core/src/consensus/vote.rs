//! Vote structures for PoI consensus (BFT-style)

use serde::{Deserialize, Serialize};
use crate::identity::GyId;

/// Vote type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteType {
    /// Pre-vote (first round of BFT)
    PreVote,
    /// Pre-commit (second round of BFT, triggers finality)
    PreCommit,
}

impl std::fmt::Display for VoteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoteType::PreVote => write!(f, "PreVote"),
            VoteType::PreCommit => write!(f, "PreCommit"),
        }
    }
}

/// A single vote cast by a validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// Voter's GyID
    pub voter: GyId,
    /// Block height
    pub height: u64,
    /// Consensus round
    pub round: u32,
    /// Vote type
    pub vote_type: VoteType,
    /// Block hash being voted on (None = nil vote)
    pub block_hash: Option<[u8; 32]>,
    /// Vote signature
    pub signature: Vec<u8>,
    /// Timestamp
    pub timestamp: i64,
}

impl Vote {
    /// Create a new unsigned vote
    pub fn new(
        voter: GyId,
        height: u64,
        round: u32,
        vote_type: VoteType,
        block_hash: Option<[u8; 32]>,
    ) -> Self {
        Self {
            voter,
            height,
            round,
            vote_type,
            block_hash,
            signature: Vec::new(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Compute the canonical bytes to sign
    pub fn sign_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.voter.id.as_bytes());
        bytes.extend_from_slice(&self.height.to_le_bytes());
        bytes.extend_from_slice(&self.round.to_le_bytes());
        bytes.push(self.vote_type as u8);
        if let Some(h) = &self.block_hash {
            bytes.extend_from_slice(h);
        }
        bytes
    }

    /// Is this a nil vote (no block hash)?
    pub fn is_nil(&self) -> bool {
        self.block_hash.is_none()
    }
}

/// Aggregated vote set for a consensus round
#[derive(Debug, Clone, Default)]
pub struct VoteSet {
    pre_votes: Vec<Vote>,
    pre_commits: Vec<Vote>,
    /// Quorum threshold (number of votes needed)
    quorum: usize,
}

impl VoteSet {
    pub fn new(quorum: usize) -> Self {
        Self {
            quorum,
            ..Default::default()
        }
    }

    /// Add a vote
    pub fn add(&mut self, vote: Vote) {
        match vote.vote_type {
            VoteType::PreVote => {
                // Deduplicate by voter
                if !self.pre_votes.iter().any(|v| v.voter == vote.voter) {
                    self.pre_votes.push(vote);
                }
            }
            VoteType::PreCommit => {
                if !self.pre_commits.iter().any(|v| v.voter == vote.voter) {
                    self.pre_commits.push(vote);
                }
            }
        }
    }

    /// Has pre-vote quorum been reached for a specific block hash?
    pub fn has_prevote_quorum(&self, block_hash: &[u8; 32]) -> bool {
        let count = self.pre_votes.iter()
            .filter(|v| v.block_hash.as_ref() == Some(block_hash))
            .count();
        count >= self.quorum
    }

    /// Has pre-commit quorum been reached?
    pub fn has_precommit_quorum(&self, block_hash: &[u8; 32]) -> bool {
        let count = self.pre_commits.iter()
            .filter(|v| v.block_hash.as_ref() == Some(block_hash))
            .count();
        count >= self.quorum
    }

    /// Total pre-votes received
    pub fn prevote_count(&self) -> usize {
        self.pre_votes.len()
    }

    /// Total pre-commits received
    pub fn precommit_count(&self) -> usize {
        self.pre_commits.len()
    }

    /// Get all pre-votes
    pub fn prevotes(&self) -> &[Vote] {
        &self.pre_votes
    }

    /// Get all pre-commits
    pub fn precommits(&self) -> &[Vote] {
        &self.pre_commits
    }
}

/// One full consensus round at a given height
#[derive(Debug, Clone)]
pub struct ConsensusRound {
    /// Block height
    pub height: u64,
    /// Round number
    pub round: u32,
    /// Proposed block hash (set by leader)
    pub proposed_block_hash: Option<[u8; 32]>,
    /// Vote set
    pub votes: VoteSet,
    /// Is this round finalized?
    pub finalized: bool,
}

impl ConsensusRound {
    pub fn new(height: u64, round: u32) -> Self {
        Self {
            height,
            round,
            proposed_block_hash: None,
            votes: VoteSet::default(),
            finalized: false,
        }
    }

    /// Set the proposed block hash from the leader
    pub fn set_proposal(&mut self, block_hash: [u8; 32]) {
        self.proposed_block_hash = Some(block_hash);
    }

    /// Add a vote to this round
    pub fn add_vote(&mut self, vote: Vote) {
        self.votes.add(vote);
    }

    /// Check if the round can be finalized (pre-commit quorum)
    pub fn can_finalize(&self) -> bool {
        if let Some(hash) = &self.proposed_block_hash {
            self.votes.has_precommit_quorum(hash)
        } else {
            false
        }
    }

    /// Mark as finalized
    pub fn finalize(&mut self) {
        self.finalized = true;
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
    fn test_vote_set_quorum() {
        let mut vs = VoteSet::new(3); // quorum = 3

        let block_hash = [1u8; 32];

        for i in 0u8..3 {
            let vote = Vote::new(
                gyid(&format!("GyIDtest{:04}", i)),
                1, 0,
                VoteType::PreVote,
                Some(block_hash),
            );
            vs.add(vote);
        }

        assert!(vs.has_prevote_quorum(&block_hash));
        assert_eq!(vs.prevote_count(), 3);
    }

    #[test]
    fn test_vote_dedup() {
        let mut vs = VoteSet::new(2);
        let block_hash = [1u8; 32];
        let id = gyid("GyIDaaaa");

        // Add same voter twice
        vs.add(Vote::new(id.clone(), 1, 0, VoteType::PreVote, Some(block_hash)));
        vs.add(Vote::new(id, 1, 0, VoteType::PreVote, Some(block_hash)));

        assert_eq!(vs.prevote_count(), 1);
    }

    #[test]
    fn test_consensus_round_finalize() {
        let mut round = ConsensusRound::new(1, 0);
        let block_hash = [42u8; 32];

        round.set_proposal(block_hash);
        round.votes = VoteSet::new(2);

        for i in 0u8..2 {
            round.add_vote(Vote::new(
                gyid(&format!("GyIDtest{:04}", i)),
                1, 0,
                VoteType::PreCommit,
                Some(block_hash),
            ));
        }

        assert!(round.can_finalize());
        round.finalize();
        assert!(round.finalized);
    }
}
