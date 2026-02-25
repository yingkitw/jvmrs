//! Deterministic execution mode for real-time, safety-critical, and blockchain systems.
//!
//! When enabled: fixed RNG seed, reproducible timestamps, no non-deterministic syscalls.
//! Used for: blockchain/smart contracts, reproducible builds, audit trails.

use std::cell::Cell;

/// Deterministic execution configuration
#[derive(Debug, Clone)]
pub struct DeterministicConfig {
    /// Fixed seed for any RNG (0 = use system time when disabled)
    pub rng_seed: u64,
    /// Use fixed "epoch" timestamp instead of actual time
    pub fixed_timestamp_ns: Option<u64>,
    /// Target max pause time in ns (for real-time / HFT; advisory)
    pub max_pause_ns: Option<u64>,
}

impl Default for DeterministicConfig {
    fn default() -> Self {
        Self {
            rng_seed: 42,
            fixed_timestamp_ns: Some(0),
            max_pause_ns: None,
        }
    }
}

impl DeterministicConfig {
    /// Preset for blockchain / smart contract execution
    pub fn blockchain() -> Self {
        Self {
            rng_seed: 0,
            fixed_timestamp_ns: Some(0),
            max_pause_ns: None,
        }
    }

    /// Preset for real-time systems with pause time target
    pub fn realtime(max_pause_ns: u64) -> Self {
        Self {
            rng_seed: 42,
            fixed_timestamp_ns: None,
            max_pause_ns: Some(max_pause_ns),
        }
    }

    /// Preset for high-frequency trading (low latency target)
    pub fn hft() -> Self {
        Self::realtime(1_000_000) // 1ms target
    }
}

/// Deterministic mode controller
pub struct DeterministicMode {
    enabled: bool,
    config: DeterministicConfig,
}

impl DeterministicMode {
    pub fn new(config: DeterministicConfig) -> Self {
        Self {
            enabled: false,
            config,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get deterministic RNG seed (or 0 if disabled)
    pub fn rng_seed(&self) -> u64 {
        if self.enabled {
            self.config.rng_seed
        } else {
            0
        }
    }

    /// Get current "time" in nanoseconds - deterministic when enabled
    pub fn timestamp_ns(&self) -> u64 {
        if self.enabled {
            self.config.fixed_timestamp_ns.unwrap_or(0)
        } else {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64
        }
    }
}

impl Default for DeterministicMode {
    fn default() -> Self {
        Self::new(DeterministicConfig::default())
    }
}
