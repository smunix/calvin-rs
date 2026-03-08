//! Structured data storage for the Calvin language.
//!
//! This module provides memory-mapped file-based storage for structured data,
//! similar to the hobbes fregion/storage system. It supports writing typed
//! data series to files and reading them back for analysis.

pub mod file;
pub mod series;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during storage operations.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File format error: {0}")]
    Format(String),

    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    #[error("Region not found: {0}")]
    RegionNotFound(String),

    #[error("Series not found: {0}")]
    SeriesNotFound(String),

    #[error("Storage full")]
    StorageFull,
}

/// The type descriptor for stored data.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StorageType {
    Unit,
    Bool,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    String,
    Array(Box<StorageType>),
    FixedArray(Box<StorageType>, usize),
    Record(BTreeMap<String, StorageType>),
    Variant(BTreeMap<String, StorageType>),
}

impl StorageType {
    /// Get the fixed size of this type in bytes, if applicable.
    pub fn fixed_size(&self) -> Option<usize> {
        match self {
            StorageType::Unit => Some(0),
            StorageType::Bool => Some(1),
            StorageType::Byte => Some(1),
            StorageType::Short => Some(2),
            StorageType::Int => Some(4),
            StorageType::Long => Some(8),
            StorageType::Float => Some(4),
            StorageType::Double => Some(8),
            StorageType::FixedArray(elem, n) => elem.fixed_size().map(|s| s * n),
            StorageType::Record(fields) => {
                let mut total = 0;
                for ty in fields.values() {
                    total += ty.fixed_size()?;
                }
                Some(total)
            }
            _ => None,
        }
    }

    /// Check if this type has a fixed (known at compile time) size.
    pub fn is_fixed_size(&self) -> bool {
        self.fixed_size().is_some()
    }
}

/// Metadata for a storage file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileMetadata {
    /// The file format version.
    pub version: u32,
    /// The creation timestamp (Unix epoch seconds).
    pub created_at: u64,
    /// The last modified timestamp.
    pub modified_at: u64,
    /// Custom metadata key-value pairs.
    pub properties: BTreeMap<String, String>,
    /// The data series stored in this file.
    pub series: BTreeMap<String, SeriesMetadata>,
}

/// Metadata for a data series within a storage file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SeriesMetadata {
    /// The name of the series.
    pub name: String,
    /// The type of each element in the series.
    pub element_type: StorageType,
    /// The number of elements stored.
    pub count: u64,
    /// The byte offset of the series data in the file.
    pub offset: u64,
    /// The byte length of the series data.
    pub length: u64,
}

/// A storage group that manages multiple series.
#[derive(Debug)]
pub struct StorageGroup {
    /// The name of this storage group.
    pub name: String,
    /// The directory where data files are stored.
    pub directory: PathBuf,
    /// The metadata for the current file.
    pub metadata: FileMetadata,
}

impl StorageGroup {
    /// Create a new storage group.
    pub fn new(name: &str, directory: &Path) -> Result<Self, StorageError> {
        std::fs::create_dir_all(directory)?;

        let metadata = FileMetadata {
            version: 1,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            modified_at: 0,
            properties: BTreeMap::new(),
            series: BTreeMap::new(),
        };

        Ok(StorageGroup {
            name: name.to_string(),
            directory: directory.to_path_buf(),
            metadata,
        })
    }

    /// Get the path to the data file for this group.
    pub fn data_file_path(&self) -> PathBuf {
        self.directory.join(format!("{}.calvin", self.name))
    }

    /// Get the path to the metadata file for this group.
    pub fn metadata_file_path(&self) -> PathBuf {
        self.directory.join(format!("{}.meta.json", self.name))
    }

    /// Save the metadata to disk.
    pub fn save_metadata(&self) -> Result<(), StorageError> {
        let json = serde_json::to_string_pretty(&self.metadata)
            .map_err(|e| StorageError::Format(e.to_string()))?;
        std::fs::write(self.metadata_file_path(), json)?;
        Ok(())
    }

    /// Load metadata from disk.
    pub fn load_metadata(name: &str, directory: &Path) -> Result<Self, StorageError> {
        let meta_path = directory.join(format!("{}.meta.json", name));
        let json = std::fs::read_to_string(&meta_path)?;
        let metadata: FileMetadata =
            serde_json::from_str(&json).map_err(|e| StorageError::Format(e.to_string()))?;

        Ok(StorageGroup {
            name: name.to_string(),
            directory: directory.to_path_buf(),
            metadata,
        })
    }

    /// List all series in this group.
    pub fn list_series(&self) -> Vec<&SeriesMetadata> {
        self.metadata.series.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_type_fixed_size() {
        assert_eq!(StorageType::Int.fixed_size(), Some(4));
        assert_eq!(StorageType::Long.fixed_size(), Some(8));
        assert_eq!(StorageType::Bool.fixed_size(), Some(1));
        assert_eq!(
            StorageType::FixedArray(Box::new(StorageType::Int), 10).fixed_size(),
            Some(40)
        );
        assert_eq!(StorageType::String.fixed_size(), None);
        assert_eq!(
            StorageType::Array(Box::new(StorageType::Int)).fixed_size(),
            None
        );
    }

    #[test]
    fn test_storage_group_creation() {
        let dir = std::env::temp_dir().join("calvin_test_storage");
        let group = StorageGroup::new("test", &dir);
        assert!(group.is_ok());
        let group = group.unwrap();
        assert_eq!(group.name, "test");
        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_metadata_serialization() {
        let dir = std::env::temp_dir().join("calvin_test_meta");
        let group = StorageGroup::new("test_meta", &dir).unwrap();
        assert!(group.save_metadata().is_ok());

        let loaded = StorageGroup::load_metadata("test_meta", &dir);
        assert!(loaded.is_ok());
        assert_eq!(loaded.unwrap().metadata.version, 1);
        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }
}
