//! Data series for structured storage.
//!
//! A series is a typed sequence of values that can be appended to and
//! read from a storage file. This is the primary mechanism for recording
//! structured data from applications.

use super::{SeriesMetadata, StorageError, StorageType};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

/// A writer for appending values to a data series.
pub struct SeriesWriter {
    /// The series metadata.
    metadata: SeriesMetadata,
    /// The buffer for pending writes.
    buffer: Vec<u8>,
    /// The number of elements written.
    count: u64,
}

impl SeriesWriter {
    /// Create a new series writer.
    pub fn new(name: &str, element_type: StorageType) -> Self {
        SeriesWriter {
            metadata: SeriesMetadata {
                name: name.to_string(),
                element_type,
                count: 0,
                offset: 0,
                length: 0,
            },
            buffer: Vec::new(),
            count: 0,
        }
    }

    /// Write an integer value.
    pub fn write_int(&mut self, value: i32) -> Result<(), StorageError> {
        self.buffer
            .write_i32::<LittleEndian>(value)
            .map_err(|e| StorageError::Io(e))?;
        self.count += 1;
        Ok(())
    }

    /// Write a long value.
    pub fn write_long(&mut self, value: i64) -> Result<(), StorageError> {
        self.buffer
            .write_i64::<LittleEndian>(value)
            .map_err(|e| StorageError::Io(e))?;
        self.count += 1;
        Ok(())
    }

    /// Write a double value.
    pub fn write_double(&mut self, value: f64) -> Result<(), StorageError> {
        self.buffer
            .write_f64::<LittleEndian>(value)
            .map_err(|e| StorageError::Io(e))?;
        self.count += 1;
        Ok(())
    }

    /// Write a boolean value.
    pub fn write_bool(&mut self, value: bool) -> Result<(), StorageError> {
        self.buffer.push(if value { 1 } else { 0 });
        self.count += 1;
        Ok(())
    }

    /// Write a byte value.
    pub fn write_byte(&mut self, value: u8) -> Result<(), StorageError> {
        self.buffer.push(value);
        self.count += 1;
        Ok(())
    }

    /// Write a string value (length-prefixed).
    pub fn write_string(&mut self, value: &str) -> Result<(), StorageError> {
        let bytes = value.as_bytes();
        self.buffer
            .write_u64::<LittleEndian>(bytes.len() as u64)
            .map_err(|e| StorageError::Io(e))?;
        self.buffer.extend_from_slice(bytes);
        self.count += 1;
        Ok(())
    }

    /// Write raw bytes.
    pub fn write_bytes(&mut self, data: &[u8]) -> Result<(), StorageError> {
        self.buffer.extend_from_slice(data);
        Ok(())
    }

    /// Get the current buffer contents.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Get the number of elements written.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Get the metadata for this series.
    pub fn metadata(&self) -> &SeriesMetadata {
        &self.metadata
    }

    /// Finalize the writer and return the buffer and metadata.
    pub fn finalize(mut self, offset: u64) -> (Vec<u8>, SeriesMetadata) {
        self.metadata.count = self.count;
        self.metadata.offset = offset;
        self.metadata.length = self.buffer.len() as u64;
        (self.buffer, self.metadata)
    }
}

/// A reader for reading values from a data series.
pub struct SeriesReader<'a> {
    /// The series metadata.
    metadata: &'a SeriesMetadata,
    /// A cursor over the raw data.
    cursor: Cursor<&'a [u8]>,
    /// The number of elements read so far.
    read_count: u64,
}

impl<'a> SeriesReader<'a> {
    /// Create a new series reader.
    pub fn new(metadata: &'a SeriesMetadata, data: &'a [u8]) -> Self {
        SeriesReader {
            metadata,
            cursor: Cursor::new(data),
            read_count: 0,
        }
    }

    /// Read an integer value.
    pub fn read_int(&mut self) -> Result<i32, StorageError> {
        let val = self
            .cursor
            .read_i32::<LittleEndian>()
            .map_err(|e| StorageError::Io(e))?;
        self.read_count += 1;
        Ok(val)
    }

    /// Read a long value.
    pub fn read_long(&mut self) -> Result<i64, StorageError> {
        let val = self
            .cursor
            .read_i64::<LittleEndian>()
            .map_err(|e| StorageError::Io(e))?;
        self.read_count += 1;
        Ok(val)
    }

    /// Read a double value.
    pub fn read_double(&mut self) -> Result<f64, StorageError> {
        let val = self
            .cursor
            .read_f64::<LittleEndian>()
            .map_err(|e| StorageError::Io(e))?;
        self.read_count += 1;
        Ok(val)
    }

    /// Read a boolean value.
    pub fn read_bool(&mut self) -> Result<bool, StorageError> {
        let val = self
            .cursor
            .read_u8()
            .map_err(|e| StorageError::Io(e))?;
        self.read_count += 1;
        Ok(val != 0)
    }

    /// Read a string value.
    pub fn read_string(&mut self) -> Result<String, StorageError> {
        let len = self
            .cursor
            .read_u64::<LittleEndian>()
            .map_err(|e| StorageError::Io(e))? as usize;
        let pos = self.cursor.position() as usize;
        let data = self.cursor.get_ref();
        if pos + len > data.len() {
            return Err(StorageError::Format("String extends beyond data".to_string()));
        }
        let s = String::from_utf8(data[pos..pos + len].to_vec())
            .map_err(|e| StorageError::Format(e.to_string()))?;
        self.cursor.set_position((pos + len) as u64);
        self.read_count += 1;
        Ok(s)
    }

    /// Check if there are more elements to read.
    pub fn has_next(&self) -> bool {
        self.read_count < self.metadata.count
    }

    /// Get the number of elements read so far.
    pub fn read_count(&self) -> u64 {
        self.read_count
    }

    /// Get the total number of elements.
    pub fn total_count(&self) -> u64 {
        self.metadata.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_and_read_ints() {
        let mut writer = SeriesWriter::new("test_ints", StorageType::Int);
        writer.write_int(1).unwrap();
        writer.write_int(2).unwrap();
        writer.write_int(3).unwrap();

        let (data, metadata) = writer.finalize(0);
        assert_eq!(metadata.count, 3);

        let mut reader = SeriesReader::new(&metadata, &data);
        assert_eq!(reader.read_int().unwrap(), 1);
        assert_eq!(reader.read_int().unwrap(), 2);
        assert_eq!(reader.read_int().unwrap(), 3);
        assert!(!reader.has_next());
    }

    #[test]
    fn test_write_and_read_strings() {
        let mut writer = SeriesWriter::new("test_strings", StorageType::String);
        writer.write_string("hello").unwrap();
        writer.write_string("world").unwrap();

        let (data, metadata) = writer.finalize(0);
        assert_eq!(metadata.count, 2);

        let mut reader = SeriesReader::new(&metadata, &data);
        assert_eq!(reader.read_string().unwrap(), "hello");
        assert_eq!(reader.read_string().unwrap(), "world");
    }

    #[test]
    fn test_write_and_read_doubles() {
        let mut writer = SeriesWriter::new("test_doubles", StorageType::Double);
        writer.write_double(3.14).unwrap();
        writer.write_double(2.718).unwrap();

        let (data, metadata) = writer.finalize(0);
        let mut reader = SeriesReader::new(&metadata, &data);
        assert!((reader.read_double().unwrap() - 3.14).abs() < 1e-10);
        assert!((reader.read_double().unwrap() - 2.718).abs() < 1e-10);
    }
}
