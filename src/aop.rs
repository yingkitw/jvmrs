//! Proxy-based AOP (Aspect-Oriented Programming) support at runtime.
//!
//! Foundation for method interception via proxy objects.

/// AOP advice kind
#[derive(Debug, Clone, Copy)]
pub enum AdviceKind {
    Before,
    After,
    Around,
}

/// Pointcut - describes where to apply advice
#[derive(Debug, Clone)]
pub struct Pointcut {
    pub class_pattern: String,
    pub method_pattern: String,
}

/// Advice registration (stub - full implementation would wire into method dispatch)
#[derive(Debug)]
pub struct AopRegistry {
    pointcuts: Vec<(Pointcut, AdviceKind)>,
}

impl AopRegistry {
    pub fn new() -> Self {
        Self {
            pointcuts: Vec::new(),
        }
    }

    pub fn add_advice(&mut self, pointcut: Pointcut, kind: AdviceKind) {
        self.pointcuts.push((pointcut, kind));
    }

    /// Check if method matches any pointcut
    pub fn matches(&self, class_name: &str, method_name: &str) -> Option<AdviceKind> {
        for (pc, kind) in &self.pointcuts {
            if (pc.class_pattern == "*" || class_name.contains(&pc.class_pattern))
                && (pc.method_pattern == "*" || method_name == pc.method_pattern)
            {
                return Some(*kind);
            }
        }
        None
    }
}

impl Default for AopRegistry {
    fn default() -> Self {
        Self::new()
    }
}
