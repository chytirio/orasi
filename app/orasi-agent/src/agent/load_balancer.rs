//! Load balancing implementation for the Orasi Agent

use crate::health::HealthStatus;
use crate::types::AgentLoad;
use std::collections::HashMap;

/// Load balancing strategy
#[derive(Debug, Clone, PartialEq)]
pub enum LoadBalancingStrategy {
    /// Round-robin load balancing
    RoundRobin,
    /// Health-based load balancing (prefer healthy nodes)
    HealthBased,
    /// Load-based load balancing (prefer less loaded nodes)
    LoadBased,
    /// Hybrid load balancing (combine health and load)
    Hybrid,
}

/// Load balancer for distributing tasks across cluster members
pub struct LoadBalancer {
    /// Load balancing strategy
    strategy: LoadBalancingStrategy,

    /// Current round-robin index
    round_robin_index: usize,

    /// Cluster member health status
    member_health: HashMap<String, HealthStatus>,

    /// Cluster member load metrics
    member_load: HashMap<String, AgentLoad>,
}

impl LoadBalancer {
    /// Create new load balancer
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            strategy,
            round_robin_index: 0,
            member_health: HashMap::new(),
            member_load: HashMap::new(),
        }
    }

    /// Update member health status
    pub fn update_member_health(&mut self, member_id: String, health: HealthStatus) {
        self.member_health.insert(member_id, health);
    }

    /// Update member load metrics
    pub fn update_member_load(&mut self, member_id: String, load: AgentLoad) {
        self.member_load.insert(member_id, load);
    }

    /// Remove member from load balancer
    pub fn remove_member(&mut self, member_id: &str) {
        self.member_health.remove(member_id);
        self.member_load.remove(member_id);
    }

    /// Select best member for task assignment
    pub fn select_member(&mut self, available_members: &[String]) -> Option<String> {
        if available_members.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalancingStrategy::RoundRobin => self.select_round_robin(available_members),
            LoadBalancingStrategy::HealthBased => self.select_health_based(available_members),
            LoadBalancingStrategy::LoadBased => self.select_load_based(available_members),
            LoadBalancingStrategy::Hybrid => self.select_hybrid(available_members),
        }
    }

    /// Round-robin selection
    fn select_round_robin(&mut self, available_members: &[String]) -> Option<String> {
        if available_members.is_empty() {
            return None;
        }

        let selected = available_members[self.round_robin_index % available_members.len()].clone();
        self.round_robin_index = (self.round_robin_index + 1) % available_members.len();
        Some(selected)
    }

    /// Health-based selection
    fn select_health_based(&self, available_members: &[String]) -> Option<String> {
        let mut healthy_members = Vec::new();
        let mut degraded_members = Vec::new();

        for member_id in available_members {
            if let Some(health) = self.member_health.get(member_id) {
                match health {
                    HealthStatus::Healthy => healthy_members.push(member_id.clone()),
                    HealthStatus::Degraded => degraded_members.push(member_id.clone()),
                    HealthStatus::Unhealthy => {
                        // Skip unhealthy members
                        continue;
                    }
                    HealthStatus::Unknown => {
                        // If unknown, assume healthy
                        healthy_members.push(member_id.clone());
                    }
                }
            } else {
                // If no health info available, assume healthy
                healthy_members.push(member_id.clone());
            }
        }

        // Prefer healthy members, fall back to degraded
        if !healthy_members.is_empty() {
            Some(healthy_members[0].clone())
        } else if !degraded_members.is_empty() {
            Some(degraded_members[0].clone())
        } else {
            None
        }
    }

    /// Load-based selection
    fn select_load_based(&self, available_members: &[String]) -> Option<String> {
        let mut best_member = None;
        let mut lowest_load = f64::MAX;

        for member_id in available_members {
            if let Some(load) = self.member_load.get(member_id) {
                // Calculate load score (weighted combination of CPU, memory, and queue length)
                let cpu_weight = 0.4;
                let memory_weight = 0.3;
                let queue_weight = 0.3;

                let cpu_score = load.cpu_percent / 100.0;
                let memory_score = load.memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0); // Normalize to GB
                let queue_score = load.queue_length as f64 / 100.0; // Normalize to reasonable queue size

                let total_load = cpu_score * cpu_weight
                    + memory_score * memory_weight
                    + queue_score * queue_weight;

                if total_load < lowest_load {
                    lowest_load = total_load;
                    best_member = Some(member_id.clone());
                }
            } else {
                // If no load info available, assume low load
                return Some(member_id.clone());
            }
        }

        best_member
    }

    /// Hybrid selection (combine health and load)
    fn select_hybrid(&self, available_members: &[String]) -> Option<String> {
        let mut healthy_members = Vec::new();
        let mut degraded_members = Vec::new();

        // First, categorize members by health
        for member_id in available_members {
            if let Some(health) = self.member_health.get(member_id) {
                match health {
                    HealthStatus::Healthy => healthy_members.push(member_id.clone()),
                    HealthStatus::Degraded => degraded_members.push(member_id.clone()),
                    HealthStatus::Unhealthy => {
                        // Skip unhealthy members
                        continue;
                    }
                    HealthStatus::Unknown => {
                        // If unknown, assume healthy
                        healthy_members.push(member_id.clone());
                    }
                }
            } else {
                // If no health info available, assume healthy
                healthy_members.push(member_id.clone());
            }
        }

        // Select best member from healthy group, then degraded group
        let candidate_groups = [&healthy_members, &degraded_members];

        for group in &candidate_groups {
            if !group.is_empty() {
                if let Some(best_member) = self.select_load_based(group) {
                    return Some(best_member);
                }
            }
        }

        None
    }

    /// Get load balancer statistics
    pub fn get_stats(&self) -> LoadBalancerStats {
        let total_members = self.member_health.len();
        let healthy_members = self
            .member_health
            .values()
            .filter(|h| matches!(h, HealthStatus::Healthy))
            .count();
        let degraded_members = self
            .member_health
            .values()
            .filter(|h| matches!(h, HealthStatus::Degraded))
            .count();
        let unhealthy_members = self
            .member_health
            .values()
            .filter(|h| matches!(h, HealthStatus::Unhealthy))
            .count();

        LoadBalancerStats {
            strategy: self.strategy.clone(),
            total_members,
            healthy_members,
            degraded_members,
            unhealthy_members,
            round_robin_index: self.round_robin_index,
        }
    }

    /// Set load balancing strategy
    pub fn set_strategy(&mut self, strategy: LoadBalancingStrategy) {
        self.strategy = strategy;
    }
}

/// Load balancer statistics
#[derive(Debug, Clone)]
pub struct LoadBalancerStats {
    pub strategy: LoadBalancingStrategy,
    pub total_members: usize,
    pub healthy_members: usize,
    pub degraded_members: usize,
    pub unhealthy_members: usize,
    pub round_robin_index: usize,
}
