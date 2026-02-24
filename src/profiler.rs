//! Integrated profiler with flame graph export and hotspot detection.
//!
//! Records method invocation counts, wall-clock time, and call stacks.
//! Exports in collapsed stack format for Brendan Gregg's flamegraph.pl.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// Sample: single call stack capture with duration
#[derive(Debug, Clone)]
pub struct ProfileSample {
    pub stack: Vec<String>,
    pub duration_ns: u64,
}

/// Integrated profiler for method-level and instruction-level profiling
pub struct Profiler {
    /// Method invocation counts
    method_counts: Mutex<HashMap<String, u64>>,
    /// Method total time (nanoseconds)
    method_time_ns: Mutex<HashMap<String, u64>>,
    /// Call stack samples for flame graph
    samples: Mutex<Vec<ProfileSample>>,
    /// Current call stack (thread-local conceptually; simplified for single-thread)
    call_stack: Mutex<Vec<String>>,
    /// Whether profiling is enabled
    enabled: bool,
    /// Maximum samples to keep (0 = unlimited)
    max_samples: usize,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            method_counts: Mutex::new(HashMap::new()),
            method_time_ns: Mutex::new(HashMap::new()),
            samples: Mutex::new(Vec::new()),
            call_stack: Mutex::new(Vec::new()),
            enabled: true,
            max_samples: 100_000,
        }
    }

    pub fn with_max_samples(max_samples: usize) -> Self {
        let mut p = Self::new();
        p.max_samples = max_samples;
        p
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Record method entry - call at start of method execution
    pub fn method_enter(&self, class_name: &str, method_name: &str) -> Option<Instant> {
        if !self.enabled {
            return None;
        }
        let full_name = format!("{}.{}", class_name, method_name);
        if let Ok(mut counts) = self.method_counts.lock() {
            *counts.entry(full_name.clone()).or_insert(0) += 1;
        }
        if let Ok(mut stack) = self.call_stack.lock() {
            stack.push(full_name);
        }
        Some(Instant::now())
    }

    /// Record method exit - call at end of method execution
    pub fn method_exit(&self, class_name: &str, method_name: &str, start: Option<Instant>) {
        if !self.enabled {
            return;
        }
        let full_name = format!("{}.{}", class_name, method_name);
        if let Some(instant) = start {
            let elapsed = instant.elapsed().as_nanos() as u64;
            if let Ok(mut time_ns) = self.method_time_ns.lock() {
                *time_ns.entry(full_name.clone()).or_insert(0) += elapsed;
            }
            if let Ok(mut samples) = self.samples.lock() {
                if samples.len() < self.max_samples {
                    if let Ok(stack) = self.call_stack.lock() {
                        samples.push(ProfileSample {
                            stack: stack.clone(),
                            duration_ns: elapsed,
                        });
                    }
                }
            }
        }
        if let Ok(mut stack) = self.call_stack.lock() {
            if stack.last().map(|s| s.as_str()) == Some(&full_name) {
                stack.pop();
            }
        }
    }

    /// Get method invocation counts
    pub fn method_counts(&self) -> HashMap<String, u64> {
        self.method_counts.lock().unwrap().clone()
    }

    /// Get method total time (ns)
    pub fn method_time_ns(&self) -> HashMap<String, u64> {
        self.method_time_ns.lock().unwrap().clone()
    }

    /// Get hotspot methods (sorted by time, descending)
    pub fn hotspots(&self, top_n: usize) -> Vec<(String, u64, u64)> {
        let time_ns = self.method_time_ns.lock().unwrap();
        let counts = self.method_counts.lock().unwrap();
        let mut rows: Vec<_> = time_ns
            .iter()
            .map(|(name, ns)| (name.clone(), *ns, counts.get(name).copied().unwrap_or(0)))
            .collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1));
        rows.into_iter().take(top_n).collect()
    }

    /// Export in collapsed stack format for flamegraph.pl
    /// Format: semicolon-separated stack; whitespace; value
    pub fn export_flame_graph(&self) -> String {
        let samples = self.samples.lock().unwrap();
        let mut stack_counts: HashMap<String, u64> = HashMap::new();
        for sample in samples.iter() {
            let key = sample.stack.join(";");
            *stack_counts.entry(key).or_insert(0) += sample.duration_ns / 1000; // use µs for readability
        }
        stack_counts
            .iter()
            .map(|(stack, count)| format!("{} {}", stack, count))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Write flame graph data to file
    pub fn write_flame_graph(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.export_flame_graph())
    }

    /// Summary report
    pub fn summary(&self) -> String {
        let time_ns = self.method_time_ns.lock().unwrap();
        let counts = self.method_counts.lock().unwrap();
        let total_time: u64 = time_ns.values().sum();
        let total_calls: u64 = counts.values().sum();
        let mut lines = vec![
            format!("Total method time: {} µs", total_time / 1000),
            format!("Total invocations: {}", total_calls),
            "".to_string(),
            "Top 10 hotspots (by time):".to_string(),
        ];
        let mut sorted: Vec<_> = time_ns.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (name, ns) in sorted.into_iter().take(10) {
            let count = counts.get(name).copied().unwrap_or(0);
            lines.push(format!("  {}: {} µs ({} calls)", name, ns / 1000, count));
        }
        lines.join("\n")
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard - calls method_exit on drop
pub struct ProfileGuard<'a> {
    profiler: &'a Profiler,
    class_name: String,
    method_name: String,
    start: Option<Instant>,
}

impl<'a> ProfileGuard<'a> {
    pub fn new(profiler: &'a Profiler, class_name: &str, method_name: &str) -> Self {
        let start = profiler.method_enter(class_name, method_name);
        Self {
            profiler,
            class_name: class_name.to_string(),
            method_name: method_name.to_string(),
            start,
        }
    }
}

impl Drop for ProfileGuard<'_> {
    fn drop(&mut self) {
        self.profiler.method_exit(&self.class_name, &self.method_name, self.start);
    }
}
