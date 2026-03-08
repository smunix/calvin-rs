//! File-based storage using memory-mapped I/O.
//!
//! This module provides low-level file operations for reading and writing
//! structured data using memory-mapped files, similar to hobbes fregion.

use super::StorageError;
use memmap2::{MmapMut, MmapOptions};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

/// The magic number for Calvin storage files.
const MAGIC: &[u8; 8] = b"CALVINDB";

/// The current file format version.
const FORMAT_VERSION: u32 = 1;

/// Header size in bytes.
const HEADER_SIZE: usize = 56;

/// A file header for Calvin storage files.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileHeader {
    /// Magic number to identify the file format.
    pub magic: [u8; 8],
    /// File format version.
    pub version: u32,
    /// Flags (reserved for future use).
    pub flags: u32,
    /// The number of data regions in the file.
    pub region_count: u64,
    /// The total data size in bytes.
    pub data_size: u64,
    /// The offset to the metadata section.
    pub metadata_offset: u64,
    /// The metadata section size.
    pub metadata_size: u64,
    /// Reserved padding.
    pub reserved: [u8; 8],
}

impl FileHeader {
    /// Create a new file header with default values.
    pub fn new() -> Self {
        let mut magic = [0u8; 8];
        magic.copy_from_slice(MAGIC);
        FileHeader {
            magic,
            version: FORMAT_VERSION,
            flags: 0,
            region_count: 0,
            data_size: 0,
            metadata_offset: HEADER_SIZE as u64,
            metadata_size: 0,
            reserved: [0u8; 8],
        }
    }

    /// Validate the file header.
    pub fn validate(&self) -> Result<(), StorageError> {
        if &self.magic != MAGIC {
            return Err(StorageError::Format("Invalid magic number".to_string()));
        }
        if self.version > FORMAT_VERSION {
            return Err(StorageError::Format(format!(
                "Unsupported version: {}",
                self.version
            )));
        }
        Ok(())
    }

    /// Serialize the header to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_SIZE);
        buf.extend_from_slice(&self.magic);
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf.extend_from_slice(&self.flags.to_le_bytes());
        buf.extend_from_slice(&self.region_count.to_le_bytes());
        buf.extend_from_slice(&self.data_size.to_le_bytes());
        buf.extend_from_slice(&self.metadata_offset.to_le_bytes());
        buf.extend_from_slice(&self.metadata_size.to_le_bytes());
        buf.extend_from_slice(&self.reserved);
        buf
    }

    /// Deserialize a header from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, StorageError> {
        if data.len() < HEADER_SIZE {
            return Err(StorageError::Format("Header too short".to_string()));
        }
        let mut magic = [0u8; 8];
        magic.copy_from_slice(&data[0..8]);
        let version = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let flags = u32::from_le_bytes(data[12..16].try_into().unwrap());
        let region_count = u64::from_le_bytes(data[16..24].try_into().unwrap());
        let data_size = u64::from_le_bytes(data[24..32].try_into().unwrap());
        let metadata_offset = u64::from_le_bytes(data[32..40].try_into().unwrap());
        let metadata_size = u64::from_le_bytes(data[40..48].try_into().unwrap());
        let mut reserved = [0u8; 8];
        reserved.copy_from_slice(&data[48..56]);

        let header = FileHeader {
            magic,
            version,
            flags,
            region_count,
            data_size,
            metadata_offset,
            metadata_size,
            reserved,
        };
        header.validate()?;
        Ok(header)
    }
}

impl Default for FileHeader {
    fn default() -> Self {
        Self::new()
    }
}

/// A memory-mapped storage file for reading and writing structured data.
pub struct StorageFile {
    /// The underlying file.
    file: File,
    /// The memory-mapped region (if active).
    mmap: Option<MmapMut>,
    /// The file path.
    path: std::path::PathBuf,
    /// The file header.
    header: FileHeader,
}

impl StorageFile {
    /// Create a new storage file at the given path.
    pub fn create(path: &Path, initial_size: u64) -> Result<Self, StorageError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        let total_size = HEADER_SIZE as u64 + initial_size;
        file.set_len(total_size)?;

        let header = FileHeader::new();

        let mut storage = StorageFile {
            file,
            mmap: None,
            path: path.to_path_buf(),
            header,
        };

        // Write the header
        storage.write_header()?;

        // Memory-map the file
        storage.map()?;

        Ok(storage)
    }

    /// Open an existing storage file.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;

        // Read the header
        let mut header_buf = [0u8; HEADER_SIZE];
        {
            use std::io::Read;
            let mut f = File::open(path)?;
            f.read_exact(&mut header_buf)?;
        }
        let header = FileHeader::from_bytes(&header_buf)?;

        let mut storage = StorageFile {
            file,
            mmap: None,
            path: path.to_path_buf(),
            header,
        };

        storage.map()?;

        Ok(storage)
    }

    /// Memory-map the file.
    fn map(&mut self) -> Result<(), StorageError> {
        let mmap = unsafe { MmapOptions::new().map_mut(&self.file)? };
        self.mmap = Some(mmap);
        Ok(())
    }

    /// Write the header to the file.
    fn write_header(&mut self) -> Result<(), StorageError> {
        let header_bytes = self.header.to_bytes();
        let mut f = &self.file;
        use std::io::Seek;
        f.seek(std::io::SeekFrom::Start(0))?;
        f.write_all(&header_bytes)?;
        f.flush()?;
        Ok(())
    }

    /// Write data at a specific offset.
    pub fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<(), StorageError> {
        if let Some(ref mut mmap) = self.mmap {
            let start = offset as usize;
            let end = start + data.len();
            if end > mmap.len() {
                return Err(StorageError::StorageFull);
            }
            mmap[start..end].copy_from_slice(data);
            Ok(())
        } else {
            Err(StorageError::Format("File not mapped".to_string()))
        }
    }

    /// Read data from a specific offset.
    pub fn read_at(&self, offset: u64, length: usize) -> Result<&[u8], StorageError> {
        if let Some(ref mmap) = self.mmap {
            let start = offset as usize;
            let end = start + length;
            if end > mmap.len() {
                return Err(StorageError::Format("Read beyond file end".to_string()));
            }
            Ok(&mmap[start..end])
        } else {
            Err(StorageError::Format("File not mapped".to_string()))
        }
    }

    /// Flush changes to disk.
    pub fn flush(&self) -> Result<(), StorageError> {
        if let Some(ref mmap) = self.mmap {
            mmap.flush()?;
        }
        Ok(())
    }

    /// Get the file header.
    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    /// Get the file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the total file size.
    pub fn size(&self) -> u64 {
        self.mmap.as_ref().map(|m| m.len() as u64).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_file_header_roundtrip() {
        let header = FileHeader::new();
        let bytes = header.to_bytes();
        let parsed = FileHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.version, FORMAT_VERSION);
        assert_eq!(&parsed.magic, MAGIC);
    }

    #[test]
    fn test_create_and_open_file() {
        let path = temp_dir().join("calvin_test_file.calvin");
        {
            let file = StorageFile::create(&path, 1024);
            assert!(file.is_ok());
        }
        {
            let file = StorageFile::open(&path);
            assert!(file.is_ok());
            let file = file.unwrap();
            assert_eq!(file.header().version, FORMAT_VERSION);
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_write_and_read() {
        let path = temp_dir().join("calvin_test_rw.calvin");
        let mut file = StorageFile::create(&path, 1024).unwrap();
        let data = b"Hello, Calvin!";
        let offset = HEADER_SIZE as u64;
        file.write_at(offset, data).unwrap();
        let read_back = file.read_at(offset, data.len()).unwrap();
        assert_eq!(read_back, data);
        let _ = std::fs::remove_file(&path);
    }
}
