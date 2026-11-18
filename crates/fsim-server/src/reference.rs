//! Reference oracle for `fsim-server` password hashing, atomic safe file writes, and checkpoint integrity.

use ring::pbkdf2;
use std::num::NonZeroU32;
use std::path::Path;

const PBKDF2_ITERATIONS: u32 = 100_000;

/// Hashes a password with a 16-byte salt using `ring::pbkdf2` (PBKDF2-HMAC-SHA256) and encodes as hex.
pub fn reference_hash_password(password: &str, salt_hex: &str) -> String {
    let mut out = [0u8; 32];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(PBKDF2_ITERATIONS).unwrap(),
        salt_hex.as_bytes(),
        password.as_bytes(),
        &mut out,
    );
    hex_encode(&out)
}

pub fn reference_verify_password(password: &str, salt_hex: &str, expected_hash_hex: &str) -> bool {
    let actual = reference_hash_password(password, salt_hex);
    actual == expected_hash_hex
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Atomic file write: writes to `<path>.tmp`, flushes, and renames to `<path>` so a crash never leaves a half-written file.
pub fn atomic_write_file(path: &Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp_path = path.with_extension("tmp");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp_path).map_err(|e| e.to_string())?;
        f.write_all(contents).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp_path, path).map_err(|e| e.to_string())
}
