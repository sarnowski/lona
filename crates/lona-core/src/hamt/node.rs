// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Internal node types and operations for the HAMT.
//!
//! Contains the `Node` and `Child` enums, along with all the recursive
//! operations for get, insert, and remove.

use alloc::rc::Rc;
use alloc::vec::Vec;

use core::hash::Hash;

use crate::fnv::FnvHasher;

/// Number of bits used per trie level.
const BITS: u32 = 5;

/// Mask for extracting bits at each level (0x1F = 31).
const MASK_U64: u64 = 31;

/// Maximum trie depth (64 bits / 5 bits per level = ~13 levels).
const MAX_DEPTH: u32 = 13;

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
fn bitmap_to_index(bitmap: u32, bit: u32) -> usize {
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

/// Inserts into a collision node.
///
/// When keys have the same hash, they're stored together in a collision
/// node. This function either updates an existing key or adds a new one.
///
/// Returns `(new_node, was_added)` where `was_added` is `true` if a new
/// entry was added rather than updating an existing one.
fn insert_into_collision<K, V>(
    collision_hash: u64,
    entries: &[(K, V)],
    key: K,
    value: V,
) -> (Node<K, V>, bool)
where
    K: Clone + Eq,
    V: Clone,
{
    let mut new_entries: Vec<(K, V)> = entries.to_vec();
    let mut found = false;

    for &mut (ref entry_key, ref mut entry_value) in &mut new_entries {
        if *entry_key == key {
            *entry_value = value.clone();
            found = true;
            break;
        }
    }

    if !found {
        new_entries.push((key, value));
    }

    let new_node = Node::Collision {
        hash: collision_hash,
        entries: Rc::from(new_entries),
    };
    (new_node, !found)
}

/// Recursively inserts a key-value pair into the trie.
///
/// Navigates through the trie using hash bits at each depth, creating
/// new nodes along the path with path copying for immutability.
///
/// Returns `(new_node, was_added)` where `was_added` is `true` if a new
/// entry was added rather than updating an existing one.
pub(super) fn insert_rec<K, V>(
    node: &Node<K, V>,
    key: K,
    value: V,
    hash: u64,
    depth: u32,
) -> (Node<K, V>, bool)
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    match *node {
        Node::Branch {
            bitmap,
            ref children,
        } => insert_into_branch(bitmap, children, key, value, hash, depth),
        Node::Collision {
            hash: collision_hash,
            ref entries,
        } => insert_into_collision(collision_hash, entries, key, value),
    }
}

/// Inserts into a branch node.
///
/// Handles the three cases for branch insertion:
/// 1. Empty slot: creates a new leaf
/// 2. Existing leaf: may replace, create collision, or expand to sub-trie
/// 3. Existing sub-node: recursively inserts
///
/// Returns `(new_node, was_added)` where `was_added` is `true` if a new
/// entry was added rather than updating an existing one.
fn insert_into_branch<K, V>(
    bitmap: u32,
    children: &Rc<[Child<K, V>]>,
    key: K,
    value: V,
    hash: u64,
    depth: u32,
) -> (Node<K, V>, bool)
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    let idx = index_at_depth(hash, depth);
    let bit = 1_u32 << idx;
    let child_idx = bitmap_to_index(bitmap, idx);

    if bitmap & bit == 0 {
        // No entry at this index, add new leaf
        let new_leaf = Child::Leaf { hash, key, value };
        let mut new_children: Vec<Child<K, V>> = children.iter().cloned().collect();
        new_children.insert(child_idx, new_leaf);
        let new_node = Node::Branch {
            bitmap: bitmap | bit,
            children: Rc::from(new_children),
        };
        return (new_node, true);
    }

    // Entry exists at this index
    let mut new_children: Vec<Child<K, V>> = children.iter().cloned().collect();
    let Some(existing_child) = new_children.get(child_idx) else {
        return (
            Node::Branch {
                bitmap,
                children: Rc::clone(children),
            },
            false,
        );
    };

    let (new_child, added) = match *existing_child {
        Child::Leaf {
            hash: leaf_hash,
            key: ref leaf_key,
            value: ref leaf_value,
        } => insert_at_leaf(leaf_hash, leaf_key, leaf_value, key, value, hash, depth),
        Child::Node(ref sub_node) => {
            let (new_sub, added) = insert_rec(sub_node, key, value, hash, depth.saturating_add(1));
            (Child::Node(Rc::new(new_sub)), added)
        }
    };

    if let Some(slot) = new_children.get_mut(child_idx) {
        *slot = new_child;
    }
    let new_node = Node::Branch {
        bitmap,
        children: Rc::from(new_children),
    };
    (new_node, added)
}

/// Handles insertion at a leaf node.
///
/// When inserting at an existing leaf, there are three possibilities:
/// - Same key: replace the value (no size change)
/// - Same hash, different key: create a collision node
/// - Different hash: expand into a sub-trie via [`merge_leaves`]
///
/// Returns `(new_child, was_added)` where `was_added` is `true` if a new
/// entry was added rather than updating an existing one.
fn insert_at_leaf<K, V>(
    leaf_hash: u64,
    leaf_key: &K,
    leaf_value: &V,
    key: K,
    value: V,
    hash: u64,
    depth: u32,
) -> (Child<K, V>, bool)
where
    K: Clone + Eq,
    V: Clone,
{
    if *leaf_key == key {
        // Replace existing value
        (Child::Leaf { hash, key, value }, false)
    } else if leaf_hash == hash {
        // Hash collision - create collision node
        let collision = Node::Collision {
            hash,
            entries: Rc::from([(leaf_key.clone(), leaf_value.clone()), (key, value)]),
        };
        (Child::Node(Rc::new(collision)), true)
    } else {
        // Different hashes - expand to sub-trie
        let existing_leaf = Child::Leaf {
            hash: leaf_hash,
            key: leaf_key.clone(),
            value: leaf_value.clone(),
        };
        let new_leaf = Child::Leaf { hash, key, value };
        let sub_node = merge_leaves(existing_leaf, new_leaf, depth.saturating_add(1));
        (Child::Node(Rc::new(sub_node)), true)
    }
}

/// Merges two leaves into a sub-trie at the given depth.
///
/// When two leaves have different hash values but collide at the same
/// position, they are recursively expanded into a sub-trie until their
/// hash bits diverge. If they share all hash bits (extremely unlikely
/// with a good hash function), they become a collision node.
///
/// # Panics
///
/// Debug builds will panic if called with non-leaf children, as this
/// indicates a bug in the HAMT implementation.
fn merge_leaves<K, V>(leaf1: Child<K, V>, leaf2: Child<K, V>, depth: u32) -> Node<K, V>
where
    K: Clone + Eq,
    V: Clone,
{
    // Extract leaf data, with defensive handling for non-leaf children
    let (hash1, key1, value1) = match leaf1 {
        Child::Leaf { hash, key, value } => (hash, key, value),
        Child::Node(_) => {
            debug_assert!(false, "merge_leaves called with non-leaf child");
            // In release builds, return an empty branch as a safe fallback
            return Node::Branch {
                bitmap: 0,
                children: Rc::from([]),
            };
        }
    };

    let (hash2, key2, value2) = match leaf2 {
        Child::Leaf { hash, key, value } => (hash, key, value),
        Child::Node(_) => {
            debug_assert!(false, "merge_leaves called with non-leaf child");
            return Node::Branch {
                bitmap: 0,
                children: Rc::from([]),
            };
        }
    };

    // If we've exhausted all hash bits, create a collision node
    if depth > MAX_DEPTH {
        return Node::Collision {
            hash: hash1,
            entries: Rc::from([(key1, value1), (key2, value2)]),
        };
    }

    let idx1 = index_at_depth(hash1, depth);
    let idx2 = index_at_depth(hash2, depth);

    // Reconstruct leaves from extracted values
    let new_leaf1 = Child::Leaf {
        hash: hash1,
        key: key1,
        value: value1,
    };
    let new_leaf2 = Child::Leaf {
        hash: hash2,
        key: key2,
        value: value2,
    };

    if idx1 == idx2 {
        // Same index at this level - recurse deeper
        let sub_node = merge_leaves(new_leaf1, new_leaf2, depth.saturating_add(1));
        let bit = 1_u32 << idx1;
        Node::Branch {
            bitmap: bit,
            children: Rc::from([Child::Node(Rc::new(sub_node))]),
        }
    } else {
        // Different indices - create branch with both leaves
        let (bit1, bit2) = (1_u32 << idx1, 1_u32 << idx2);
        let bitmap = bit1 | bit2;
        let children = if idx1 < idx2 {
            Rc::from([new_leaf1, new_leaf2])
        } else {
            Rc::from([new_leaf2, new_leaf1])
        };
        Node::Branch { bitmap, children }
    }
}

/// Result of a remove operation.
pub(super) enum RemoveResult<K, V> {
    /// Key was not found.
    NotFound,
    /// Key was removed; the Option contains the new node (None if node should be removed).
    Removed(Option<Node<K, V>>),
}

/// Recursively removes a key from the trie.
///
/// Navigates through the trie using hash bits to locate the entry,
/// then removes it and restructures the trie as needed (collapsing
/// single-child branches, converting collision nodes back to leaves).
pub(super) fn remove_rec<K, V, Q>(
    node: &Node<K, V>,
    key: &Q,
    hash: u64,
    depth: u32,
) -> RemoveResult<K, V>
where
    K: Clone + core::borrow::Borrow<Q>,
    V: Clone,
    Q: Eq + ?Sized,
{
    match *node {
        Node::Branch {
            bitmap,
            ref children,
        } => remove_from_branch(bitmap, children, key, hash, depth),
        Node::Collision {
            hash: collision_hash,
            ref entries,
        } => remove_from_collision(collision_hash, entries, key, depth),
    }
}

/// Removes a key from a branch node.
///
/// Handles removal from branch nodes, including recursive removal from
/// sub-nodes and collapsing empty children from the bitmap.
fn remove_from_branch<K, V, Q>(
    bitmap: u32,
    children: &Rc<[Child<K, V>]>,
    key: &Q,
    hash: u64,
    depth: u32,
) -> RemoveResult<K, V>
where
    K: Clone + core::borrow::Borrow<Q>,
    V: Clone,
    Q: Eq + ?Sized,
{
    let idx = index_at_depth(hash, depth);
    let bit = 1_u32 << idx;

    if bitmap & bit == 0 {
        return RemoveResult::NotFound;
    }

    let child_idx = bitmap_to_index(bitmap, idx);
    let Some(child) = children.get(child_idx) else {
        return RemoveResult::NotFound;
    };

    match *child {
        Child::Leaf {
            key: ref leaf_key, ..
        } => {
            if leaf_key.borrow() != key {
                return RemoveResult::NotFound;
            }
            remove_child_at(bitmap, bit, child_idx, children)
        }
        Child::Node(ref sub_node) => {
            match remove_rec(sub_node, key, hash, depth.saturating_add(1)) {
                RemoveResult::NotFound => RemoveResult::NotFound,
                RemoveResult::Removed(None) => remove_child_at(bitmap, bit, child_idx, children),
                RemoveResult::Removed(Some(new_sub)) => {
                    let mut new_children: Vec<Child<K, V>> = children.iter().cloned().collect();
                    if let Some(slot) = new_children.get_mut(child_idx) {
                        *slot = Child::Node(Rc::new(new_sub));
                    }
                    RemoveResult::Removed(Some(Node::Branch {
                        bitmap,
                        children: Rc::from(new_children),
                    }))
                }
            }
        }
    }
}

/// Removes a child at the given index from a branch.
///
/// Updates the bitmap and children array to remove the specified child.
/// Returns `Removed(None)` if this leaves the branch empty, otherwise
/// returns the new branch node.
fn remove_child_at<K, V>(
    bitmap: u32,
    bit: u32,
    child_idx: usize,
    children: &Rc<[Child<K, V>]>,
) -> RemoveResult<K, V>
where
    K: Clone,
    V: Clone,
{
    let new_bitmap = bitmap ^ bit;
    if new_bitmap == 0 {
        return RemoveResult::Removed(None);
    }

    let mut new_children: Vec<Child<K, V>> = children.iter().cloned().collect();
    let _removed = new_children.remove(child_idx);
    RemoveResult::Removed(Some(Node::Branch {
        bitmap: new_bitmap,
        children: Rc::from(new_children),
    }))
}

/// Removes a key from a collision node.
///
/// If removal leaves only one entry, converts back to a leaf wrapped
/// in a branch. If the collision becomes empty, returns `Removed(None)`.
fn remove_from_collision<K, V, Q>(
    collision_hash: u64,
    entries: &Rc<[(K, V)]>,
    key: &Q,
    depth: u32,
) -> RemoveResult<K, V>
where
    K: Clone + core::borrow::Borrow<Q>,
    V: Clone,
    Q: Eq + ?Sized,
{
    let mut new_entries: Vec<(K, V)> = Vec::with_capacity(entries.len());
    let mut found = false;

    for &(ref entry_key, ref entry_value) in entries.iter() {
        if entry_key.borrow() == key {
            found = true;
        } else {
            new_entries.push((entry_key.clone(), entry_value.clone()));
        }
    }

    if !found {
        return RemoveResult::NotFound;
    }

    if new_entries.is_empty() {
        RemoveResult::Removed(None)
    } else if new_entries.len() == 1 {
        let (remaining_key, remaining_value) = new_entries.remove(0);
        let idx = index_at_depth(collision_hash, depth);
        let bit = 1_u32 << idx;
        RemoveResult::Removed(Some(Node::Branch {
            bitmap: bit,
            children: Rc::from([Child::Leaf {
                hash: collision_hash,
                key: remaining_key,
                value: remaining_value,
            }]),
        }))
    } else {
        RemoveResult::Removed(Some(Node::Collision {
            hash: collision_hash,
            entries: Rc::from(new_entries),
        }))
    }
}
