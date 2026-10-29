//! The SHA-256 of a file's contents, as hex: what the step cache keys a
//! file by and the Downloads watcher takes a download once by.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

/// The SHA-256 of the file at `path`, read in 64 KiB chunks.
///
/// # Errors
///
/// Fails when the file cannot be opened or read.
pub fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_hashes_to_the_sha256_of_its_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("a.txt");
        std::fs::write(&file, b"abc").expect("write");
        assert_eq!(
            hash_file(&file).expect("hash"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(hash_file(&dir.path().join("missing")).is_err());
    }
}
