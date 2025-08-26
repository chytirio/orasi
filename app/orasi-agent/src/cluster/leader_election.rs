//! Leader election implementation for cluster coordination

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Leader election state
#[derive(Debug, Clone)]
pub struct LeaderElectionState {
    /// Current election ID
    pub current_election_id: Option<String>,

    /// Election start time
    pub election_start_time: Option<u64>,

    /// Election timeout
    pub election_timeout: u64,

    /// Candidate votes received
    pub votes_received: HashMap<String, String>, // candidate_id -> voter_id

    /// Election status
    pub election_status: ElectionStatus,

    /// Last election timestamp
    pub last_election_timestamp: u64,
}

/// Election status
#[derive(Debug, Clone, PartialEq)]
pub enum ElectionStatus {
    Idle,
    Campaigning,
    Elected,
    Following,
}

impl Default for LeaderElectionState {
    fn default() -> Self {
        Self {
            current_election_id: None,
            election_start_time: None,
            election_timeout: 60_000, // 60 seconds
            votes_received: HashMap::new(),
            election_status: ElectionStatus::Idle,
            last_election_timestamp: 0,
        }
    }
}

/// Leader election message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderElectionMessage {
    /// Election ID
    pub election_id: String,

    /// Candidate ID
    pub candidate_id: String,

    /// Election timestamp
    pub timestamp: u64,

    /// Election type
    pub election_type: ElectionType,

    /// Voter ID (for vote messages)
    pub voter_id: Option<String>,

    /// Vote result (for result messages)
    pub vote_result: Option<bool>,
}

/// Election type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElectionType {
    Start,
    Vote,
    Result,
    Resign,
}

impl LeaderElectionState {
    /// Start a new election
    pub fn start_election(&mut self, election_id: String) {
        self.current_election_id = Some(election_id.clone());
        self.election_start_time = Some(super::types::current_timestamp());
        self.election_status = ElectionStatus::Campaigning;
        self.votes_received.clear();
        self.last_election_timestamp = super::types::current_timestamp();
    }

    /// Record a vote
    pub fn record_vote(&mut self, candidate_id: String, voter_id: String) {
        self.votes_received.insert(candidate_id, voter_id);
    }

    /// Check if election has timed out
    pub fn has_election_timed_out(&self) -> bool {
        if let Some(start_time) = self.election_start_time {
            let current_time = super::types::current_timestamp();
            current_time - start_time > self.election_timeout
        } else {
            false
        }
    }

    /// Get vote count for a candidate
    pub fn get_vote_count(&self, candidate_id: &str) -> usize {
        self.votes_received
            .values()
            .filter(|voter_id| *voter_id == candidate_id)
            .count()
    }

    /// Complete election
    pub fn complete_election(&mut self, elected: bool) {
        self.election_status = if elected {
            ElectionStatus::Elected
        } else {
            ElectionStatus::Following
        };
        self.current_election_id = None;
        self.election_start_time = None;
    }

    /// Resign leadership
    pub fn resign_leadership(&mut self) {
        self.election_status = ElectionStatus::Idle;
        self.current_election_id = None;
        self.election_start_time = None;
        self.votes_received.clear();
    }
}
