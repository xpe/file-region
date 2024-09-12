# file-region

This crate provides a `FileRegion` type which encapsulates a particular region of a `File`.

## Example

You can find this example at `examples/basic.rs` and run it with `cargo run --example basic`.

```rust
use file_region::{FileRegion, FileRegionError};
use std::io::{Read, Seek, SeekFrom, Write};
use tempfile::tempfile;

fn main() -> Result<(), FileRegionError> {
    let mut file = tempfile()?;
    file.write_all(b"Hello, FileRegion.")?;

    let mut region = FileRegion::new(&file, 7..16);
    let mut buffer = [0; 9];
    region.read(0, &mut buffer)?;
    assert_eq!(&buffer, b"FileRegio");

    region.write(0, b"01234")?;

    let mut content = String::new();
    file.seek(SeekFrom::Start(0))?;
    file.read_to_string(&mut content)?;
    assert_eq!(content, "Hello, 01234egion.");

    Ok(())
}
```
