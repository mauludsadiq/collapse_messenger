use std::fs;
use std::io;
use std::path::PathBuf;

use crate::types::Digest;

fn cas_dir() -> PathBuf {
    PathBuf::from(".cas")
}

fn digest_to_hex(d: &Digest) -> String {
    d.0.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn put(bytes: &[u8]) -> io::Result<Digest> {
    use crate::types::compute_digest;

    let digest = compute_digest(&bytes);
    let dir = cas_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join(digest_to_hex(&digest));
    if !path.exists() {
        fs::write(&path, bytes)?;
    }
    Ok(digest)
}

pub fn get(digest: &Digest) -> io::Result<Vec<u8>> {
    let dir = cas_dir();
    let path = dir.join(digest_to_hex(digest));
    let data = fs::read(path)?;
    Ok(data)
}
