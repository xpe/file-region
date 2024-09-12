use std::fs::{File, Metadata};
use std::io::ErrorKind::InvalidInput;
use std::io::{self, Error, Read, Seek, SeekFrom, Write};
use std::ops::Range;

pub struct FileRegion<'a> {
    file: &'a File,
    range: Range<u64>,
}

impl<'a> FileRegion<'a> {
    pub fn file_metadata(&self) -> io::Result<Metadata> {
        self.file.metadata()
    }

    pub fn range(self) -> Range<u64> {
        self.range
    }

    pub fn len(&self) -> u64 {
        self.range.end - self.range.start
    }

    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    pub fn new(file: &File, range: Range<u64>) -> FileRegion {
        FileRegion { file, range }
    }

    pub fn is_valid(&self) -> io::Result<bool> {
        let metadata = self.file.metadata()?;
        let file_len = metadata.len();
        Ok(self.range.start <= file_len && self.range.end <= file_len)
    }

    pub fn from_file(file: &'a File) -> io::Result<FileRegion<'a>> {
        let range = 0..file.metadata()?.len();
        Ok(FileRegion { file, range })
    }

    /// Return a subregion. Checks for some inconsistencies but not all; use
    /// `is_valid()` to check consistency against the underlying file.
    pub fn subregion(self, range: Range<u64>) -> io::Result<FileRegion<'a>> {
        let start = {
            let s = self.range.start.checked_add(range.start);
            s.ok_or_else(|| Error::new(InvalidInput, "subregion start overflow"))?
        };
        let end = {
            let e = self.range.start.checked_add(range.end);
            e.ok_or_else(|| Error::new(InvalidInput, "subregion end overflow"))?
        };
        if start > self.range.end {
            return Err(Error::new(InvalidInput, "subregion start exceeds parent"));
        }
        if end > self.range.end {
            return Err(Error::new(InvalidInput, "subregion end exceeds parent"));
        }
        Ok(FileRegion {
            file: self.file,
            range: start..end,
        })
    }

    pub fn read(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        self.file.seek(SeekFrom::Start(self.range.start + offset))?;
        let max = self.len().saturating_sub(offset);
        (&mut self.file).take(max).read(buf)
    }

    pub fn write(&mut self, offset: u64, buf: &[u8]) -> io::Result<usize> {
        self.file.seek(SeekFrom::Start(self.range.start + offset))?;
        let x = self.len().saturating_sub(offset).try_into();
        let max: usize = x.map_err(|_| Error::new(InvalidInput, "offset too large"))?;
        let buf_max = max.min(buf.len());
        self.file.write(&buf[..buf_max])
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, SeekFrom, Write};
    use tempfile::tempfile;

    use super::FileRegion;

    #[test]
    fn test_from_file() {
        let mut file = tempfile().unwrap();
        file.write_all(b"Hello, World!").unwrap();
        file.flush().unwrap();
        let fr = FileRegion::from_file(&file).unwrap();
        assert_eq!(fr.range(), 0..13);
    }

    #[test]
    fn test_is_valid_true() {
        let mut file = tempfile().unwrap();
        file.write_all(b"0123456789").unwrap();
        file.flush().unwrap();
        let fr = FileRegion::from_file(&file).unwrap();
        assert!(fr.is_valid().unwrap());
    }

    #[test]
    fn test_is_valid_false_1() {
        let file = tempfile().unwrap();
        let fr = FileRegion::new(&file, 0..7);
        assert!(!fr.is_valid().unwrap());
    }

    #[test]
    fn test_is_valid_false_2() {
        let mut file = tempfile().unwrap();
        file.write_all(b"0123456789").unwrap();
        file.flush().unwrap();
        let fr = FileRegion::new(&file, 0..11);
        assert!(!fr.is_valid().unwrap());
    }

    #[test]
    fn test_subregion() {
        let file = tempfile().unwrap();
        let fr = FileRegion::new(&file, 100..2100);
        let sub = fr.subregion(200..600).unwrap();
        assert_eq!(sub.range(), 300..700);
    }

    #[test]
    fn test_write_within_region() {
        let mut file = tempfile().unwrap();
        file.write_all(&[0; 200]).unwrap();

        {
            let mut fr = FileRegion::new(&file, 100..120);
            let written = fr.write(0, b"enshittification").unwrap();
            assert_eq!(written, 16);
        }

        file.seek(SeekFrom::Start(0)).unwrap();
        let mut content = vec![0; 200];
        file.read_exact(&mut content).unwrap();

        assert_eq!(content[..100], [0; 100]);
        assert_eq!(&content[100..116], b"enshittification");
        assert_eq!(content[116..], [0; 84]);
    }

    #[test]
    fn test_write_beyond_region() {
        let mut file = tempfile().unwrap();
        file.write_all(&[0; 200]).unwrap();

        {
            let mut fr = FileRegion::new(&file, 100..110);
            let written = fr.write(0, b"enshittification").unwrap();
            assert_eq!(written, 10);
        }

        file.seek(SeekFrom::Start(0)).unwrap();
        let mut content = vec![0; 200];
        file.read_exact(&mut content).unwrap();

        assert_eq!(content[..100], [0; 100]);
        assert_eq!(&content[100..110], b"enshittifi");
        assert_eq!(content[110..], [0; 90]);
    }

    #[test]
    fn test_example() {
        let mut file = tempfile().unwrap();
        file.write_all(b"Hello, FileRegion.").unwrap();

        let mut region = FileRegion::new(&file, 7..16);
        let mut buffer = [0; 9];
        region.read(0, &mut buffer).unwrap();
        assert_eq!(&buffer, b"FileRegio");

        region.write(0, b"01234").unwrap();

        let mut content = String::new();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_to_string(&mut content).unwrap();
        assert_eq!(content, "Hello, 01234egion.");
    }
}
