use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitBreakerState {
    Closed,   // Normal operation
    Open,     // Circuit is open, requests are blocked
    HalfOpen, // Testing if service is recovered
}

/// Circuit breaker for fault tolerance
pub struct CircuitBreaker {
    failure_threshold: u32,
    timeout: Duration,
    state: AtomicU32, // 0=Closed, 1=Open, 2=HalfOpen
    failure_count: AtomicU32,
    last_failure_time: AtomicU64,
    success_count: AtomicU32,
    half_open_success_threshold: u32,
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub fn new(failure_threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            timeout,
            state: AtomicU32::new(0), // Closed
            failure_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            success_count: AtomicU32::new(0),
            half_open_success_threshold: 3,
        }
    }

    /// Check if circuit breaker allows the operation
    pub fn can_execute(&self) -> bool {
        match self.get_state() {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                // Check if timeout has passed
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                if now - last_failure >= self.timeout.as_millis() as u64 {
                    // Transition to half-open
                    self.state.store(2, Ordering::Relaxed); // HalfOpen
                    self.success_count.store(0, Ordering::Relaxed);
                    true
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    /// Record a successful operation
    pub fn record_success(&self) {
        match self.get_state() {
            CircuitBreakerState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitBreakerState::HalfOpen => {
                let success_count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if success_count >= self.half_open_success_threshold {
                    // Transition back to closed
                    self.state.store(0, Ordering::Relaxed); // Closed
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                }
            }
            CircuitBreakerState::Open => {
                // Should not happen in normal operation
            }
        }
    }

    /// Record a failed operation
    pub fn record_failure(&self) {
        match self.get_state() {
            CircuitBreakerState::Closed => {
                let failure_count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if failure_count >= self.failure_threshold {
                    // Transition to open
                    self.state.store(1, Ordering::Relaxed); // Open
                    self.last_failure_time.store(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64,
                        Ordering::Relaxed,
                    );
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Transition back to open
                self.state.store(1, Ordering::Relaxed); // Open
                self.last_failure_time.store(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    Ordering::Relaxed,
                );
                self.success_count.store(0, Ordering::Relaxed);
            }
            CircuitBreakerState::Open => {
                // Update last failure time
                self.last_failure_time.store(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    Ordering::Relaxed,
                );
            }
        }
    }

    /// Get current state
    pub fn get_state(&self) -> CircuitBreakerState {
        match self.state.load(Ordering::Relaxed) {
            0 => CircuitBreakerState::Closed,
            1 => CircuitBreakerState::Open,
            2 => CircuitBreakerState::HalfOpen,
            _ => CircuitBreakerState::Closed, // Default fallback
        }
    }

    /// Get failure count
    pub fn get_failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Relaxed)
    }

    /// Get success count (relevant in half-open state)
    pub fn get_success_count(&self) -> u32 {
        self.success_count.load(Ordering::Relaxed)
    }

    /// Reset circuit breaker to closed state
    pub fn reset(&self) {
        self.state.store(0, Ordering::Relaxed); // Closed
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        self.last_failure_time.store(0, Ordering::Relaxed);
    }

    /// Set half-open success threshold
    pub fn set_half_open_success_threshold(&mut self, threshold: u32) {
        self.half_open_success_threshold = threshold;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));
        assert_eq!(cb.get_state(), CircuitBreakerState::Closed);
        assert!(cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_failure_threshold() {
        let cb = CircuitBreaker::new(2, Duration::from_secs(60));

        // First failure
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitBreakerState::Closed);
        assert!(cb.can_execute());

        // Second failure - should open circuit
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitBreakerState::Open);
        assert!(!cb.can_execute());
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(100));

        // Trigger failure to open circuit
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitBreakerState::Open);

        // Wait for timeout and check if it transitions to half-open
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(cb.can_execute()); // This should transition to half-open
        assert_eq!(cb.get_state(), CircuitBreakerState::HalfOpen);

        // Record success to close circuit
        cb.record_success();
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.get_state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::new(1, Duration::from_secs(60));

        // Trigger failure
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitBreakerState::Open);

        // Reset
        cb.reset();
        assert_eq!(cb.get_state(), CircuitBreakerState::Closed);
        assert!(cb.can_execute());
    }
}
