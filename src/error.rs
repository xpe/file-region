use std::io::Error as IoError;

#[derive(Debug)]
pub enum FileRegionError {
    Io(IoError),
    Region(RegionError),
}

#[derive(Debug)]
pub enum RegionError {
    StartOverflow,
    EndOverflow,
    StartOutOfBounds,
    EndOutOfBounds,
}

impl From<IoError> for FileRegionError {
    fn from(error: IoError) -> Self {
        FileRegionError::Io(error)
    }
}

impl From<RegionError> for FileRegionError {
    fn from(error: RegionError) -> Self {
        FileRegionError::Region(error)
    }
}
