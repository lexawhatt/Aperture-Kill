use sha2::{Digest, Sha256};

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);

    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }

    out
}
