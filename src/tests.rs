use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use tempfile::tempfile;

use crate::{FileRegion, FileRegionError, RegionError};

fn tempfile_len_10() -> File {
    let mut file = tempfile().unwrap();
    file.write_all(b"0123456789").unwrap();
    file
}

#[test]
fn test_new_invalid() {
    let file = tempfile().unwrap();
    assert!(!FileRegion::new(&file, 0..7).is_valid().unwrap());
}

#[test]
fn test_new_write_valid() {
    let mut file = tempfile().unwrap();
    file.write_all(b"0123456789").unwrap();
    file.flush().unwrap();
    assert!(FileRegion::new(&file, 0..9).is_valid().unwrap());
    assert!(FileRegion::new(&file, 0..10).is_valid().unwrap());
}

#[test]
fn test_new_write_invalid() {
    let file = tempfile_len_10();
    assert!(!FileRegion::new(&file, 0..11).is_valid().unwrap());
    assert!(!FileRegion::new(&file, 0..12).is_valid().unwrap());
}

#[test]
fn test_try_new_ok() {
    let file = tempfile_len_10();
    assert!(FileRegion::try_new(&file, 0..5).is_ok());
}

#[test]
fn test_try_new_start_out_of_bounds() {
    let file = tempfile_len_10();
    assert!(matches!(
        FileRegion::try_new(&file, 10..15),
        Err(FileRegionError::Region(RegionError::StartOutOfBounds))
    ));
}

#[test]
fn test_try_new_end_out_of_bounds() {
    let file = tempfile_len_10();
    assert!(matches!(
        FileRegion::try_new(&file, 5..15),
        Err(FileRegionError::Region(RegionError::EndOutOfBounds))
    ));
}

#[test]
fn test_from_file_valid() {
    let file = tempfile_len_10();
    let fr = FileRegion::from_file(&file).unwrap();
    assert!(fr.is_valid().unwrap());
}

#[test]
fn test_from_file_range() {
    let mut file = tempfile().unwrap();
    file.write_all(b"Hello, World!").unwrap();
    file.flush().unwrap();
    let fr = FileRegion::from_file(&file).unwrap();
    assert_eq!(fr.range(), 0..13);
}

#[test]
fn test_file_metadata() {
    let file = tempfile_len_10();
    let region = FileRegion::new(&file, 2..6);
    let metadata = region.file_metadata().unwrap();
    assert_eq!(metadata.len(), 10);
}

#[test]
fn test_new_empty_region() {
    let file = tempfile().unwrap();
    let fr = FileRegion::new(&file, 0..0);
    assert!(fr.is_empty());
    assert_eq!(fr.len(), 0);
}

#[test]
fn test_read_in_region() {
    let file = tempfile_len_10();
    let mut fr = FileRegion::new(&file, 2..6);
    {
        let mut buf = *b"___";
        assert_eq!(fr.read(0, &mut buf).unwrap(), 3);
        assert_eq!(&buf, b"234");
    }
    {
        let mut buf = *b"___";
        assert_eq!(fr.read(1, &mut buf).unwrap(), 3);
        assert_eq!(&buf, b"345");
    }
}

#[test]
fn test_read_up_to_region_boundary() {
    let file = tempfile_len_10();
    let mut fr = FileRegion::new(&file, 2..6);
    {
        let mut buf = *b"____";
        assert_eq!(fr.read(0, &mut buf).unwrap(), 4);
        assert_eq!(&buf, b"2345");
    }
    {
        let mut buf = *b"____";
        assert_eq!(fr.read(1, &mut buf).unwrap(), 3);
        assert_eq!(&buf, b"345_");
    }
}

#[test]
fn test_start_read_offset_beyond_region() {
    let file = tempfile_len_10();
    let mut fr = FileRegion::new(&file, 3..7);
    let mut buf = [0; 2];
    assert!(matches!(
        fr.read(4, &mut buf),
        Err(FileRegionError::Region(RegionError::StartOutOfBounds))
    ));
    assert!(matches!(
        fr.read(5, &mut buf),
        Err(FileRegionError::Region(RegionError::StartOutOfBounds))
    ));
}

#[test]
fn test_read_start_overflow() {
    let file = tempfile().unwrap();
    let mut region = FileRegion::new(&file, (u64::MAX - 10)..u64::MAX);
    let mut buf = [0; 5];

    assert!(matches!(
        region.read(11, &mut buf),
        Err(FileRegionError::Region(RegionError::StartOverflow))
    ));
}

#[test]
fn test_write_in_region() {
    let mut file = tempfile().unwrap();
    file.write_all(&[0; 40]).unwrap();

    let mut fr = FileRegion::new(&file, 10..30);
    let written = fr.write(0, b"enshittification").unwrap();
    assert_eq!(written, 16);

    file.seek(SeekFrom::Start(0)).unwrap();
    let mut content = vec![0; 40];
    file.read_exact(&mut content).unwrap();

    assert_eq!(content[..10], [0; 10]);
    assert_eq!(&content[10..26], b"enshittification");
    assert_eq!(content[26..], [0; 14]);
}

#[test]
fn test_write_starting_in_region_but_too_long() {
    let mut file = tempfile().unwrap();
    file.write_all(&[0; 40]).unwrap();

    let mut fr = FileRegion::new(&file, 10..20);
    assert!(matches!(
        fr.write(0, b"enshittification"),
        Err(FileRegionError::Region(RegionError::EndOutOfBounds))
    ));
}

#[test]
fn test_write_starting_beyond_region_boundary() {
    let mut file = tempfile().unwrap();
    file.write_all(&[0; 40]).unwrap();

    let mut fr = FileRegion::new(&file, 10..20);
    assert!(matches!(
        fr.write(10, b"enshittification"),
        Err(FileRegionError::Region(RegionError::StartOutOfBounds))
    ));
}

#[test]
fn test_subregion_success() {
    let file = tempfile().unwrap();
    let fr = FileRegion::new(&file, 100..2100);
    let sub = fr.subregion(200..600).unwrap();
    assert_eq!(sub.range(), 300..700);
}

#[test]
fn test_subregion_start_overflow() {
    let file = tempfile().unwrap();
    let fr = FileRegion::new(&file, u64::MAX - 10..u64::MAX);
    assert!(matches!(
        fr.subregion(11..20),
        Err(RegionError::StartOverflow)
    ));
}

#[test]
fn test_subregion_end_overflow() {
    let file = tempfile().unwrap();
    let fr = FileRegion::new(&file, u64::MAX - 10..u64::MAX);
    assert!(matches!(fr.subregion(0..11), Err(RegionError::EndOverflow)));
}

#[test]
fn test_subregion_start_out_of_bounds() {
    let file = tempfile().unwrap();
    let fr = FileRegion::new(&file, 10..20);
    assert!(matches!(
        fr.subregion(11..15),
        Err(RegionError::StartOutOfBounds)
    ));
}

#[test]
fn test_subregion_end_out_of_bounds() {
    let file = tempfile().unwrap();
    let fr = FileRegion::new(&file, 10..20);
    assert!(matches!(
        fr.subregion(0..11),
        Err(RegionError::EndOutOfBounds)
    ));
}

#[test]
fn test_full_example() {
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
