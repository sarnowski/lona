// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Internal node types and operations for the HAMT.
//!
//! Contains the `Node` and `Child` enums, along with the recursive
//! get operation and shared helper functions.

use alloc::rc::Rc;

use core::hash::Hash;

use crate::fnv::FnvHasher;

/// Number of bits used per trie level.
const BITS: u32 = 5;

/// Mask for extracting bits at each level (0x1F = 31).
const MASK_U64: u64 = 31;

/// Maximum trie depth (64 bits / 5 bits per level = ~13 levels).
pub(super) const MAX_DEPTH: u32 = 13;

/// A node in the HAMT.
pub(super) enum Node<K, V> {
    /// A sparse internal node with a bitmap indicating which children exist.
    Branch {
        /// Bitmap where each bit indicates if the corresponding child exists.
        bitmap: u32,
        /// Compact array of children (only for bits set in bitmap).
        children: Rc<[Child<K, V>]>,
    },
    /// A collision bucket for keys with the same hash.
    Collision {
        /// The shared hash of all entries.
        hash: u64,
        /// Key-value pairs with the same hash.
        entries: Rc<[(K, V)]>,
    },
}

/// A child of a branch node.
pub(super) enum Child<K, V> {
    /// A leaf entry (single key-value pair).
    Leaf { hash: u64, key: K, value: V },
    /// A sub-trie.
    Node(Rc<Node<K, V>>),
}

impl<K: Clone, V: Clone> Clone for Node<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        match *self {
            Self::Branch {
                bitmap,
                ref children,
            } => Self::Branch {
                bitmap,
                children: Rc::clone(children),
            },
            Self::Collision { hash, ref entries } => Self::Collision {
                hash,
                entries: Rc::clone(entries),
            },
        }
    }
}

impl<K: Clone, V: Clone> Clone for Child<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        match *self {
            Self::Leaf {
                hash,
                ref key,
                ref value,
            } => Self::Leaf {
                hash,
                key: key.clone(),
                value: value.clone(),
            },
            Self::Node(ref node) => Self::Node(Rc::clone(node)),
        }
    }
}

/// Computes a hash for the given key using FNV-1a.
///
/// Uses the deterministic [`FnvHasher`] to compute a 64-bit hash value
/// for any hashable key. This is used internally by the HAMT for key
/// distribution across trie levels.
pub(super) fn hash_key<Q: Hash + ?Sized>(key: &Q) -> u64 {
    use core::hash::Hasher as _;
    let mut hasher = FnvHasher::default();
    key.hash(&mut hasher);
    hasher.finish()
}

/// Extracts the index bits at the given depth from a hash.
///
/// Each trie level uses 5 bits from the hash, starting from the least
/// significant bits at depth 0. This function extracts the appropriate
/// 5-bit chunk for the given depth.
///
/// Returns a value in the range 0..32 (5 bits).
#[inline]
pub(super) fn index_at_depth(hash: u64, depth: u32) -> u32 {
    let shift = depth.saturating_mul(BITS);
    let masked = (hash >> shift) & MASK_U64;
    // masked is at most 31, so conversion to u32 always succeeds
    u32::try_from(masked).unwrap_or(0)
}

/// Converts a bitmap position to the compact array index.
///
/// The HAMT uses a bitmap to indicate which children exist. This function
/// counts the number of set bits before the given position to determine
/// the index into the sparse children array.
#[inline]
pub(super) fn bitmap_to_index(bitmap: u32, bit: u32) -> usize {
    let mask = (1_u32 << bit).saturating_sub(1);
    let count = (bitmap & mask).count_ones();
    usize::try_from(count).unwrap_or(0)
}

/// Recursively looks up a key in the trie.
///
/// Navigates through branch nodes using hash bits at each depth until
/// finding a leaf or collision node, then checks for key equality.
pub(super) fn get_rec<'node, K, V, Q>(
    node: &'node Node<K, V>,
    key: &Q,
    hash: u64,
    depth: u32,
) -> Option<&'node V>
where
    K: core::borrow::Borrow<Q>,
    Q: Eq + ?Sized,
{
    match *node {
        Node::Branch {
            bitmap,
            ref children,
        } => {
            let idx = index_at_depth(hash, depth);
            let bit = 1_u32 << idx;

            if bitmap & bit == 0 {
                return None;
            }

            let child_idx = bitmap_to_index(bitmap, idx);
            let child = children.get(child_idx)?;

            match *child {
                Child::Leaf {
                    key: ref leaf_key,
                    value: ref leaf_value,
                    ..
                } => (leaf_key.borrow() == key).then_some(leaf_value),
                Child::Node(ref sub_node) => get_rec(sub_node, key, hash, depth.saturating_add(1)),
            }
        }
        Node::Collision { ref entries, .. } => {
            for &(ref entry_key, ref entry_value) in entries.iter() {
                if entry_key.borrow() == key {
                    return Some(entry_value);
                }
            }
            None
        }
    }
}
