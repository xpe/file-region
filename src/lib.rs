mod core;
mod error;

pub use core::FileRegion;
pub use error::{FileRegionError, RegionError};

#[cfg(test)]
mod tests;
