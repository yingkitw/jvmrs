//! Cloud and distributed computing extensions.
//!
//! Stubs for Kubernetes operator and distributed object sharing.

/// Kubernetes operator configuration (stub)
#[derive(Debug, Clone)]
pub struct K8sOperatorConfig {
    pub namespace: String,
}

impl Default for K8sOperatorConfig {
    fn default() -> Self {
        Self {
            namespace: "default".to_string(),
        }
    }
}

/// Distributed object handle (stub)
#[derive(Debug, Clone)]
pub struct DistributedObjectRef {
    pub instance_id: String,
    pub object_id: u64,
}
