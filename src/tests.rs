use std::io::{Read, Seek, SeekFrom, Write};

use tempfile::tempfile;

use crate::FileRegion;

#[test]
fn test_new_invalid() {
    let file = tempfile().unwrap();
    let fr = FileRegion::new(&file, 0..7);
    assert!(!fr.is_valid().unwrap());
}

#[test]
fn test_new_write_invalid() {
    let mut file = tempfile().unwrap();
    file.write_all(b"0123456789").unwrap();
    file.flush().unwrap();
    let fr = FileRegion::new(&file, 0..11);
    assert!(!fr.is_valid().unwrap());
}

#[test]
fn test_from_file_valid() {
    let mut file = tempfile().unwrap();
    file.write_all(b"0123456789").unwrap();
    file.flush().unwrap();
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
fn test_new_empty_region() {
    let file = tempfile().unwrap();
    let fr = FileRegion::new(&file, 0..0);
    assert!(fr.is_empty());
    assert_eq!(fr.len(), 0);
}

#[test]
fn test_read_in_region() {
    let mut file = tempfile().unwrap();
    file.write_all(b"0123456789").unwrap();
    let mut fr = FileRegion::new(&file, 3..7);
    let mut buf = [0; 4];
    assert_eq!(fr.read(0, &mut buf).unwrap(), 4);
    assert_eq!(&buf, b"3456");
}

#[test]
fn test_read_at_region_boundary() {
    let mut file = tempfile().unwrap();
    file.write_all(b"0123456789").unwrap();
    let mut fr = FileRegion::new(&file, 3..7);
    let mut buf = *b"____";
    assert_eq!(fr.read(2, &mut buf).unwrap(), 2);
    assert_eq!(&buf, b"56__");
}

#[test]
fn test_write_within_region() {
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
fn test_write_beyond_region() {
    let mut file = tempfile().unwrap();
    file.write_all(&[0; 40]).unwrap();

    let mut fr = FileRegion::new(&file, 10..20);
    let written = fr.write(0, b"enshittification").unwrap();
    assert_eq!(written, 10);

    file.seek(SeekFrom::Start(0)).unwrap();
    let mut content = vec![0; 40];
    file.read_exact(&mut content).unwrap();

    assert_eq!(content[..10], [0; 10]);
    assert_eq!(&content[10..20], b"enshittifi");
    assert_eq!(content[20..], [0; 20]);
}

#[test]
fn test_subregion() {
    let file = tempfile().unwrap();
    let fr = FileRegion::new(&file, 100..2100);
    let sub = fr.subregion(200..600).unwrap();
    assert_eq!(sub.range(), 300..700);
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
