//! JVMRS Kubernetes Operator
//!
//! Manages JVMRS-based workloads on Kubernetes.
//! Scaffold for future: reconcile JvmrsApp custom resources.

use kube::{Client, Resource};
use kube_runtime::Controller;
use tracing::info;

/// Custom resource kind for JVMRS applications
pub const JVMRS_APP_GROUP: &str = "jvmrs.io";
pub const JVMRS_APP_VERSION: &str = "v1alpha1";
pub const JVMRS_APP_KIND: &str = "JvmrsApp";

/// Run the operator (scaffold - no CRD yet)
pub async fn run(client: Client) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("JVMRS operator starting");
    // TODO: Register JvmrsApp CRD, create controller
    info!("JVMRS operator ready (no CRDs registered yet)");
    Ok(())
}
