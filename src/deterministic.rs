//! Deterministic execution mode for real-time and safety-critical systems.
//!
//! When enabled: fixed RNG seed, reproducible timestamps, no non-deterministic syscalls.

use std::cell::Cell;

/// Deterministic execution configuration
#[derive(Debug, Clone)]
pub struct DeterministicConfig {
    /// Fixed seed for any RNG (0 = use system time when disabled)
    pub rng_seed: u64,
    /// Use fixed "epoch" timestamp instead of actual time
    pub fixed_timestamp_ns: Option<u64>,
}

impl Default for DeterministicConfig {
    fn default() -> Self {
        Self {
            rng_seed: 42,
            fixed_timestamp_ns: Some(0),
        }
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
