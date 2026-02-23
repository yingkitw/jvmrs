//! Async I/O integration with Rust's async ecosystem (tokio/async-std).
//!
//! Enable with `cargo build --features async`

#[cfg(feature = "async")]
use async_trait::async_trait;
#[cfg(feature = "async")]
use tokio::fs;
#[cfg(feature = "async")]
use tokio::io::AsyncReadExt;

use crate::class_file::{ClassFile, ParseError};
use std::path::Path;

/// Async class loader - loads class files asynchronously
#[cfg(feature = "async")]
#[async_trait]
pub trait AsyncClassLoader: Send + Sync {
    /// Load a class file from path asynchronously
    async fn load_class_async(&self, path: &Path) -> Result<ClassFile, ParseError>;
}

/// Tokio-based async class loader
#[cfg(feature = "async")]
pub struct TokioClassLoader;

#[cfg(feature = "async")]
#[async_trait]
impl AsyncClassLoader for TokioClassLoader {
    async fn load_class_async(&self, path: &Path) -> Result<ClassFile, ParseError> {
        let mut data = Vec::new();
        let mut file = fs::File::open(path).await.map_err(ParseError::from)?;
        file.read_to_end(&mut data).await.map_err(ParseError::from)?;
        ClassFile::parse(&data)
    }
}
