use std::fs::{File, Metadata};
use std::io::Error as IoError;
use std::io::ErrorKind::InvalidInput;
use std::io::Result as IoResult;
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::Range;

pub struct FileRegion<'a> {
    file: &'a File,
    range: Range<u64>,
}

impl<'a> FileRegion<'a> {
    /// Creates a new `FileRegion`. Note that `range` is _not_ validated against
    /// the `file`. Use `is_valid()` to check consistency.
    pub fn new(file: &File, range: Range<u64>) -> FileRegion {
        FileRegion { file, range }
    }

    /// Creates a new `FileRegion` spanning the entire `file`. Validity is
    /// guaranteed.
    pub fn from_file(file: &'a File) -> IoResult<FileRegion<'a>> {
        let range = 0..file.metadata()?.len();
        Ok(FileRegion { file, range })
    }

    /// Returns the file metadata.
    pub fn file_metadata(&self) -> IoResult<Metadata> {
        self.file.metadata()
    }

    /// Return the region (a range).
    pub fn range(self) -> Range<u64> {
        self.range
    }

    /// Returns the length of the region in bytes.
    pub fn len(&self) -> u64 {
        self.range.end - self.range.start
    }

    /// Returns if the region is empty (zero length).
    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    /// Checks if the `FileRegion` range is valid for the underlying file.
    /// Returns true if the range is within the file's bounds, false otherwise.
    /// Performs I/O to get the file's metadata.
    pub fn is_valid(&self) -> IoResult<bool> {
        let metadata = self.file.metadata()?;
        let file_len = metadata.len();
        Ok(self.range.start <= file_len && self.range.end <= file_len)
    }

    /// Performs a bounded read operation within the file region. Returns the
    /// number of bytes successfully read.
    ///
    /// Reads data into the given buffer, limited by the region's remaining size
    /// from the offset. The actual number of bytes read may be less than the
    /// buffer's capacity.
    ///
    /// Returns an error if I/O errors occur during seeking or reading.
    pub fn read(&mut self, offset: u64, buf: &mut [u8]) -> IoResult<usize> {
        self.file.seek(SeekFrom::Start(self.range.start + offset))?;
        let max = self.len().saturating_sub(offset);
        (&mut self.file).take(max).read(buf)
    }

    /// Performs a bounded write operation within the file region. Returns the
    /// number of bytes successfully written.
    ///
    /// Writes data from the given buffer, limited by the region's size. If the
    /// buffer exceeds the available space, this is not an error; it only writes
    /// up to the region's end.
    ///
    /// Returns an error if:
    /// - `offset: u64` is too large to fit in `usize`
    /// - I/O errors occur during seeking or writing
    pub fn write(&mut self, offset: u64, buf: &[u8]) -> IoResult<usize> {
        self.file.seek(SeekFrom::Start(self.range.start + offset))?;
        let x = self.len().saturating_sub(offset).try_into();
        let max: usize = x.map_err(|_| IoError::new(InvalidInput, "offset too large"))?;
        let buf_max = max.min(buf.len());
        self.file.write(&buf[..buf_max])
    }

    /// Return a subregion. Checks for some inconsistencies but not all; use
    /// `is_valid()` to check consistency against the underlying file.
    pub fn subregion(self, range: Range<u64>) -> IoResult<FileRegion<'a>> {
        let start = self.checked_offset(range.start, "start")?;
        let end = self.checked_offset(range.end, "end")?;
        if start > self.range.end {
            return Err(IoError::new(InvalidInput, "subregion start exceeds parent"));
        }
        if end > self.range.end {
            return Err(IoError::new(InvalidInput, "subregion end exceeds parent"));
        }
        Ok(FileRegion {
            file: self.file,
            range: start..end,
        })
    }

    fn checked_offset(&self, offset: u64, operation: &str) -> IoResult<u64> {
        let n = self.range.start.checked_add(offset);
        n.ok_or_else(|| IoError::new(InvalidInput, format!("subregion {} overflow", operation)))
    }
}
