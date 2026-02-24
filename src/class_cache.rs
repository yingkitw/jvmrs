//! Fast class loading with custom binary cache format.
//!
//! Caches raw class file bytes keyed by class name for faster loading
//! (skip classpath search, read from cache dir).

use crate::class_file::ClassFile;
use byteorder::{ReadBytesExt, WriteBytesExt, BE};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const CACHE_MAGIC: u32 = 0x4A564D43; // "JVMC" = JVM Cache

/// Write class bytes to cache file
pub fn write_to_cache(cache_dir: &Path, class_name: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(cache_dir)?;
    let safe_name = class_name.replace(".", "_").replace("/", "_");
    let path = cache_dir.join(format!("{}.jvmc", safe_name));
    let mut f = std::fs::File::create(&path)?;
    f.write_u32::<BE>(CACHE_MAGIC)?;
    f.write_u32::<BE>(1)?;
    f.write_u32::<BE>(bytes.len() as u32)?;
    f.write_all(bytes)?;
    Ok(path)
}

/// Read class from cache if present
pub fn read_from_cache(cache_dir: &Path, class_name: &str) -> std::io::Result<Option<ClassFile>> {
    let safe_name = class_name.replace(".", "_").replace("/", "_");
    let path = cache_dir.join(format!("{}.jvmc", safe_name));
    if !path.exists() {
        return Ok(None);
    }
    let mut f = std::fs::File::open(path)?;
    let magic = f.read_u32::<BE>()?;
    if magic != CACHE_MAGIC {
        return Ok(None);
    }
    let _version = f.read_u32::<BE>()?;
    let len = f.read_u32::<BE>()? as usize;
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf)?;
    ClassFile::parse(&buf)
        .map(Some)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{:?}", e)))
}

/// Default cache directory
pub fn default_cache_dir() -> PathBuf {
    std::env::temp_dir().join("jvmrs_class_cache")
}
