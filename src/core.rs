use std::fs::{File, Metadata};
use std::io::Result as IoResult;
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::Range;

use super::error::{FileRegionError, RegionError};

pub struct FileRegion<'a> {
    file: &'a File,
    range: Range<u64>,
}

impl<'a> FileRegion<'a> {
    /// Creates a new `FileRegion`. Note that `range` is _not_ validated against
    /// the `file`. Use `is_valid()` or `validate()` to check consistency.
    pub fn new(file: &File, range: Range<u64>) -> FileRegion {
        FileRegion { file, range }
    }

    /// Creates a new `FileRegion`, validating the `range` against the `file`.
    /// Returns `Ok(FileRegion)` if valid. Otherwise, returns a
    /// `FileRegionError` due to invalid range or I/O errors during
    /// validation.
    pub fn try_new(file: &'a File, range: Range<u64>) -> Result<FileRegion, FileRegionError> {
        let region = FileRegion::new(file, range);
        region.validate()?;
        Ok(region)
    }

    /// Creates a new `FileRegion` spanning the entire `file`. Validity is
    /// guaranteed.
    pub fn from_file(file: &'a File) -> IoResult<Self> {
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
        let len = metadata.len();
        // Note the careful usage of `<` and `<=`.
        Ok(self.range.start < len && self.range.end <= len)
    }

    /// Validates the `FileRegion` by checking if its range is within the bounds
    /// of the underlying file. Returns `Ok(())` if valid, otherwise returns a
    /// `FileRegionError` detailing the specific validation failure or I/O error
    /// encountered.
    pub fn validate(&self) -> Result<(), FileRegionError> {
        let metadata = self.file.metadata().map_err(FileRegionError::Io)?;
        let len = metadata.len();
        validate_range(&self.range, len).map_err(FileRegionError::Region)
    }

    /// Performs a bounded read operation within the file region. Returns the
    /// number of bytes successfully read.
    ///
    /// Reads data into the given buffer, limited by the region's remaining size
    /// from the offset. The actual number of bytes read may be less than the
    /// buffer's capacity.
    ///
    /// If the read begin inside the region, no error is returned, even if the
    /// read attempts to go past the end of the region.
    ///
    /// On the other hand, if the read attempts to start beyond the region,
    /// returns an error.
    ///
    /// Returns an error if `offset: u64` is too large to fit in `usize`.
    ///
    /// May return an I/O error from seeking or reading.
    pub fn read(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize, FileRegionError> {
        let start = self
            .range
            .start
            .checked_add(offset)
            .ok_or(FileRegionError::Region(RegionError::StartOverflow))?;
        if start >= self.range.end {
            return Err(FileRegionError::Region(RegionError::StartOutOfBounds));
        }
        self.file
            .seek(SeekFrom::Start(start))
            .map_err(FileRegionError::Io)?;
        let limit = self.len().saturating_sub(offset);
        (&mut self.file)
            .take(limit)
            .read(buf)
            .map_err(FileRegionError::Io)
    }

    /// Attempts to perform a bounded write operation within the file region.
    ///
    /// Returns the number of bytes successfully written.
    ///
    /// If any part of the write are out-of-bounds, write nothing and return an
    /// error. There are two out-of-bound cases:
    /// - start the write in the region that is too long
    /// - start the write beyond the region
    ///
    /// Returns an error if `offset: u64` is too large to fit in `usize`.
    ///
    /// May return an I/O error from seeking or writing.
    pub fn write(&mut self, offset: u64, buf: &[u8]) -> Result<usize, FileRegionError> {
        let range = subrange(&self.range, offset..offset + buf.len() as u64)
            .map_err(FileRegionError::Region)?;
        self.file
            .seek(SeekFrom::Start(range.start))
            .map_err(FileRegionError::Io)?;
        self.file.write(buf).map_err(FileRegionError::Io)
    }

    /// Return a subregion. Checks for some inconsistencies but not all; use
    /// `is_valid()` to check consistency against the underlying file.
    pub fn subregion(self, range: Range<u64>) -> Result<FileRegion<'a>, RegionError> {
        Ok(FileRegion {
            file: self.file,
            range: subrange(&self.range, range)?,
        })
    }
}

fn subrange(parent: &Range<u64>, child: Range<u64>) -> Result<Range<u64>, RegionError> {
    let add = |offset: u64| parent.start.checked_add(offset);
    let start = add(child.start).ok_or(RegionError::StartOverflow)?;
    let end = add(child.end).ok_or(RegionError::EndOverflow)?;
    let range = start..end;
    validate_range(&range, parent.end)?;
    Ok(range)
}

/// Validates the range for a provided file length.
fn validate_range(range: &Range<u64>, len: u64) -> Result<(), RegionError> {
    // Note the careful usage of `>=` and `>`.
    if range.start >= len {
        Err(RegionError::StartOutOfBounds)
    } else if range.end > len {
        Err(RegionError::EndOutOfBounds)
    } else {
        Ok(())
    }
}
