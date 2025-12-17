// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! FNV-1a hasher implementation.
//!
//! Provides a simple, deterministic hasher suitable for use in hash-based
//! data structures. Unlike the standard library's hasher, this one doesn't
//! rely on OS randomness, making it appropriate for `no_std` environments.

use core::hash::Hasher;

/// FNV-1a offset basis for 64-bit hashes.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a prime for 64-bit hashes.
const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// A simple FNV-1a hasher for consistent hashing without OS randomness.
///
/// This hasher is suitable for use in hash maps and other data structures
/// in `no_std` environments where `std::collections::hash_map::RandomState`
/// is not available.
///
/// The FNV-1a algorithm is simple and fast, providing good distribution
/// for typical data patterns.
pub struct FnvHasher {
    /// The current hash state.
    hash: u64,
}

impl FnvHasher {
    /// Creates a new FNV-1a hasher initialized with the FNV offset basis.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self { hash: FNV_OFFSET }
    }
}

impl Default for FnvHasher {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for FnvHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.hash ^= u64::from(byte);
            self.hash = self.hash.wrapping_mul(FNV_PRIME);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::hash::Hash;

    #[test]
    fn new_initializes_with_offset() {
        let hasher = FnvHasher::new();
        assert_eq!(hasher.hash, FNV_OFFSET);
    }

    #[test]
    fn default_initializes_with_offset() {
        let hasher = FnvHasher::default();
        assert_eq!(hasher.hash, FNV_OFFSET);
    }

    #[test]
    fn empty_input_returns_offset() {
        let hasher = FnvHasher::new();
        // Empty write should still have offset as base
        assert_eq!(hasher.finish(), FNV_OFFSET);
    }

    #[test]
    fn consistent_hashing() {
        let mut h1 = FnvHasher::new();
        let mut h2 = FnvHasher::new();

        "hello".hash(&mut h1);
        "hello".hash(&mut h2);

        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn different_inputs_different_hashes() {
        let mut h1 = FnvHasher::new();
        let mut h2 = FnvHasher::new();

        "hello".hash(&mut h1);
        "world".hash(&mut h2);

        assert_ne!(h1.finish(), h2.finish());
    }

    #[test]
    fn write_modifies_hash() {
        let mut hasher = FnvHasher::new();
        let initial = hasher.finish();

        hasher.write(&[1, 2, 3]);

        assert_ne!(hasher.finish(), initial);
    }

    #[test]
    fn multiple_writes_accumulate() {
        let mut h1 = FnvHasher::new();
        h1.write(&[1, 2, 3, 4, 5]);

        let mut h2 = FnvHasher::new();
        h2.write(&[1, 2]);
        h2.write(&[3, 4, 5]);

        assert_eq!(h1.finish(), h2.finish());
    }
}
