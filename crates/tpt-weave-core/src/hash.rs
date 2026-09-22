//! Dependency-free FNV-1a hashing for deterministic identifiers.
//!
//! FNV-1a is used instead of `DefaultHasher` because its output is guaranteed
//! stable across Rust releases, which identifiers persisted in `.tpt-weave/`
//! artefacts require.

/// FNV-1a 64-bit offset basis.
pub const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64-bit prime.
pub const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Computes the FNV-1a 64-bit hash of `bytes`.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Computes the FNV-1a 64-bit hash of `bytes` as 16 lowercase hex characters.
pub fn fnv1a64_hex(bytes: &[u8]) -> String {
    format!("{:016x}", fnv1a64(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors_are_stable() {
        // Standard FNV-1a 64 test vectors; these must never change.
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
        assert_eq!(fnv1a64_hex(b"a"), "af63dc4c8601ec8c");
    }
}
