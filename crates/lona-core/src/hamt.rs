// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Hash Array Mapped Trie (HAMT) implementation.
//!
//! This module provides a persistent (immutable) hash map data structure using
//! a 32-way branching trie with bitmap-based sparse node representation.
//! Operations are O(log32 n), which is effectively constant for practical sizes.
//!
//! The implementation uses structural sharing to enable efficient copy-on-write
//! semantics. When a map is modified, only the path from root to the modified
//! entry is copied; the rest of the structure is shared.
//!
//! This module is internal and not exported directly. Use [`crate::map::Map`]
//! for the public API.

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
enum Node<K, V> {
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
enum Child<K, V> {
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

/// A persistent hash map using a Hash Array Mapped Trie.
///
/// Provides O(log32 n) lookup, insert, and remove operations with
/// structural sharing for efficient immutable updates.
pub struct Hamt<K, V> {
    /// Number of entries in the map.
    len: usize,
    /// Root node (None if empty).
    root: Option<Rc<Node<K, V>>>,
}

impl<K: Clone, V: Clone> Clone for Hamt<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            len: self.len,
            root: self.root.clone(),
        }
    }
}

impl<K: Clone + Eq + Hash, V: Clone> Default for Hamt<K, V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Hamt<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    /// Creates an empty HAMT.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self { len: 0, root: None }
    }

    /// Returns the number of entries in the map.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the map is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns a reference to the value for the given key.
    #[must_use]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: core::borrow::Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let hash = hash_key(key);
        let node = self.root.as_ref()?;
        get_rec(node, key, hash, 0)
    }

    /// Returns `true` if the map contains the given key.
    #[must_use]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: core::borrow::Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.get(key).is_some()
    }

    /// Returns a new map with the key-value pair inserted.
    ///
    /// If the key already exists, its value is replaced.
    #[must_use]
    pub fn insert(&self, key: K, value: V) -> Self {
        let hash = hash_key(&key);

        match self.root {
            None => {
                // Create first entry
                let leaf = Child::Leaf { hash, key, value };
                let children: Vec<Child<K, V>> = alloc::vec![leaf];
                let bitmap = 1_u32 << index_at_depth(hash, 0);
                let root = Node::Branch {
                    bitmap,
                    children: Rc::from(children),
                };
                Self {
                    len: 1,
                    root: Some(Rc::new(root)),
                }
            }
            Some(ref node) => {
                let (new_root, added) = insert_rec(node, key, value, hash, 0);
                let new_len = if added {
                    self.len.saturating_add(1)
                } else {
                    self.len
                };
                Self {
                    len: new_len,
                    root: Some(Rc::new(new_root)),
                }
            }
        }
    }

    /// Returns a new map with the key removed.
    #[must_use]
    pub fn remove<Q>(&self, key: &Q) -> Self
    where
        K: core::borrow::Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let hash = hash_key(key);

        let Some(ref node) = self.root else {
            return Self::new();
        };

        match remove_rec(node, key, hash, 0) {
            RemoveResult::NotFound => self.clone(),
            RemoveResult::Removed(None) => Self::new(),
            RemoveResult::Removed(Some(new_node)) => Self {
                len: self.len.saturating_sub(1),
                root: Some(Rc::new(new_node)),
            },
        }
    }

    /// Returns an iterator over key-value pairs.
    #[inline]
    #[must_use]
    pub fn iter(&self) -> HamtIter<'_, K, V> {
        let stack = self
            .root
            .as_ref()
            .map_or_else(Vec::new, |node| alloc::vec![IterState::Node(node)]);
        HamtIter { stack }
    }

    /// Returns an iterator over keys.
    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(key, _value)| key)
    }

    /// Returns an iterator over values.
    #[inline]
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_key, value)| value)
    }
}

/// Computes a hash for the given key using FNV-1a.
///
/// Uses the deterministic [`FnvHasher`] to compute a 64-bit hash value
/// for any hashable key. This is used internally by the HAMT for key
/// distribution across trie levels.
fn hash_key<Q: Hash + ?Sized>(key: &Q) -> u64 {
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
fn index_at_depth(hash: u64, depth: u32) -> u32 {
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
fn get_rec<'node, K, V, Q>(
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
fn insert_rec<K, V>(
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
enum RemoveResult<K, V> {
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
fn remove_rec<K, V, Q>(node: &Node<K, V>, key: &Q, hash: u64, depth: u32) -> RemoveResult<K, V>
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

/// State for the HAMT iterator.
enum IterState<'hamt, K, V> {
    /// A node to traverse.
    Node(&'hamt Node<K, V>),
    /// A child to process.
    Child(&'hamt Child<K, V>),
    /// Entries in a collision bucket.
    Collision {
        entries: &'hamt [(K, V)],
        index: usize,
    },
}

/// Iterator over HAMT entries.
pub struct HamtIter<'hamt, K, V> {
    stack: Vec<IterState<'hamt, K, V>>,
}

impl<'hamt, K, V> Iterator for HamtIter<'hamt, K, V> {
    type Item = (&'hamt K, &'hamt V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let state = self.stack.pop()?;

            match state {
                IterState::Node(node) => match *node {
                    Node::Branch { ref children, .. } => {
                        // Push children in reverse order so we iterate forward
                        for child in children.iter().rev() {
                            self.stack.push(IterState::Child(child));
                        }
                    }
                    Node::Collision { ref entries, .. } => {
                        self.stack.push(IterState::Collision { entries, index: 0 });
                    }
                },
                IterState::Child(child) => match *child {
                    Child::Leaf {
                        ref key, ref value, ..
                    } => {
                        return Some((key, value));
                    }
                    Child::Node(ref node) => {
                        self.stack.push(IterState::Node(node));
                    }
                },
                IterState::Collision { entries, index } => {
                    if let Some(&(ref key, ref value)) = entries.get(index) {
                        self.stack.push(IterState::Collision {
                            entries,
                            index: index.saturating_add(1),
                        });
                        return Some((key, value));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;
    use alloc::vec;

    #[test]
    fn new_creates_empty() {
        let hamt: Hamt<String, i32> = Hamt::new();
        assert!(hamt.is_empty());
        assert_eq!(hamt.len(), 0);
    }

    #[test]
    fn insert_single() {
        let hamt: Hamt<String, i32> = Hamt::new();
        let hamt = hamt.insert(String::from("key"), 42);

        assert_eq!(hamt.len(), 1);
        assert_eq!(hamt.get("key"), Some(&42));
    }

    #[test]
    fn insert_multiple() {
        let mut hamt: Hamt<String, i32> = Hamt::new();
        for i in 0..100 {
            let key = alloc::format!("key{i}");
            hamt = hamt.insert(key, i);
        }

        assert_eq!(hamt.len(), 100);
        for i in 0..100 {
            let key = alloc::format!("key{i}");
            assert_eq!(hamt.get(&key), Some(&i));
        }
    }

    #[test]
    fn insert_replace() {
        let hamt: Hamt<String, i32> = Hamt::new();
        let hamt = hamt.insert(String::from("key"), 1);
        let hamt = hamt.insert(String::from("key"), 2);

        assert_eq!(hamt.len(), 1);
        assert_eq!(hamt.get("key"), Some(&2));
    }

    #[test]
    fn get_missing() {
        let hamt: Hamt<String, i32> = Hamt::new();
        assert_eq!(hamt.get("missing"), None);
    }

    #[test]
    fn contains_key() {
        let hamt: Hamt<String, i32> = Hamt::new();
        let hamt = hamt.insert(String::from("exists"), 1);

        assert!(hamt.contains_key("exists"));
        assert!(!hamt.contains_key("missing"));
    }

    #[test]
    fn remove_existing() {
        let hamt: Hamt<String, i32> = Hamt::new();
        let hamt = hamt.insert(String::from("key"), 42);
        let hamt = hamt.remove("key");

        assert!(hamt.is_empty());
        assert_eq!(hamt.get("key"), None);
    }

    #[test]
    fn remove_missing() {
        let hamt: Hamt<String, i32> = Hamt::new();
        let hamt = hamt.insert(String::from("key"), 42);
        let hamt = hamt.remove("other");

        assert_eq!(hamt.len(), 1);
        assert_eq!(hamt.get("key"), Some(&42));
    }

    #[test]
    fn insert_preserves_original() {
        let h1: Hamt<String, i32> = Hamt::new();
        let h1 = h1.insert(String::from("a"), 1);
        let h2 = h1.insert(String::from("b"), 2);

        assert_eq!(h1.len(), 1);
        assert_eq!(h1.get("b"), None);
        assert_eq!(h2.len(), 2);
    }

    #[test]
    fn iter_elements() {
        let mut hamt: Hamt<i32, i32> = Hamt::new();
        for i in 0..10 {
            hamt = hamt.insert(i, i.saturating_mul(10));
        }

        let mut collected: Vec<_> = hamt.iter().map(|(&k, &v)| (k, v)).collect();
        collected.sort();

        let expected: Vec<(i32, i32)> = (0..10_i32).map(|i| (i, i.saturating_mul(10))).collect();
        assert_eq!(collected, expected);
    }

    #[test]
    fn keys_iterator() {
        let hamt: Hamt<String, i32> = Hamt::new();
        let hamt = hamt.insert(String::from("a"), 1);
        let hamt = hamt.insert(String::from("b"), 2);

        let mut keys: Vec<_> = hamt.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, vec![String::from("a"), String::from("b")]);
    }

    #[test]
    fn large_map() {
        let mut hamt: Hamt<i32, i32> = Hamt::new();
        let count = 1000;

        for i in 0..count {
            hamt = hamt.insert(i, i);
        }

        assert_eq!(hamt.len(), 1000);

        for i in 0..count {
            assert_eq!(hamt.get(&i), Some(&i));
        }
    }

    // =========================================================================
    // Collision Tests
    //
    // These tests use a custom key type that allows us to control the hash
    // value, enabling direct testing of collision handling.
    // =========================================================================

    /// A key type with a controllable hash value for testing collisions.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct CollisionKey {
        id: i32,
        forced_hash: u64,
    }

    impl CollisionKey {
        fn new(id: i32, hash: u64) -> Self {
            Self {
                id,
                forced_hash: hash,
            }
        }
    }

    impl core::hash::Hash for CollisionKey {
        fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
            // Write the forced hash value directly
            state.write_u64(self.forced_hash);
        }
    }

    #[test]
    fn collision_two_keys_same_hash() {
        // Two different keys with the same hash should both be stored
        let key1 = CollisionKey::new(1, 0xDEAD_BEEF);
        let key2 = CollisionKey::new(2, 0xDEAD_BEEF);

        let hamt: Hamt<CollisionKey, &str> = Hamt::new();
        let hamt = hamt.insert(key1.clone(), "first");
        let hamt = hamt.insert(key2.clone(), "second");

        assert_eq!(hamt.len(), 2);
        assert_eq!(hamt.get(&key1), Some(&"first"));
        assert_eq!(hamt.get(&key2), Some(&"second"));
    }

    #[test]
    fn collision_update_existing_in_collision_bucket() {
        // Updating a key in a collision bucket should work correctly
        let key1 = CollisionKey::new(1, 0xCAFE_BABE);
        let key2 = CollisionKey::new(2, 0xCAFE_BABE);

        let hamt: Hamt<CollisionKey, &str> = Hamt::new();
        let hamt = hamt.insert(key1.clone(), "first");
        let hamt = hamt.insert(key2.clone(), "second");
        let hamt = hamt.insert(key1.clone(), "updated");

        assert_eq!(hamt.len(), 2);
        assert_eq!(hamt.get(&key1), Some(&"updated"));
        assert_eq!(hamt.get(&key2), Some(&"second"));
    }

    #[test]
    fn collision_remove_from_collision_bucket() {
        // Removing from a collision bucket should leave the other entry
        let key1 = CollisionKey::new(1, 0x1234_5678);
        let key2 = CollisionKey::new(2, 0x1234_5678);

        let hamt: Hamt<CollisionKey, &str> = Hamt::new();
        let hamt = hamt.insert(key1.clone(), "first");
        let hamt = hamt.insert(key2.clone(), "second");
        let hamt = hamt.remove(&key1);

        assert_eq!(hamt.len(), 1);
        assert_eq!(hamt.get(&key1), None);
        assert_eq!(hamt.get(&key2), Some(&"second"));
    }

    #[test]
    fn collision_remove_last_from_collision_converts_to_leaf() {
        // Removing the second-to-last from collision should convert to leaf
        let key1 = CollisionKey::new(1, 0xAAAA_BBBB);
        let key2 = CollisionKey::new(2, 0xAAAA_BBBB);

        let hamt: Hamt<CollisionKey, &str> = Hamt::new();
        let hamt = hamt.insert(key1.clone(), "first");
        let hamt = hamt.insert(key2.clone(), "second");
        let hamt = hamt.remove(&key1);
        let hamt = hamt.remove(&key2);

        assert!(hamt.is_empty());
    }

    #[test]
    fn collision_three_keys_same_hash() {
        // Three different keys with the same hash
        let key1 = CollisionKey::new(1, 0x9999_9999);
        let key2 = CollisionKey::new(2, 0x9999_9999);
        let key3 = CollisionKey::new(3, 0x9999_9999);

        let hamt: Hamt<CollisionKey, i32> = Hamt::new();
        let hamt = hamt.insert(key1.clone(), 100);
        let hamt = hamt.insert(key2.clone(), 200);
        let hamt = hamt.insert(key3.clone(), 300);

        assert_eq!(hamt.len(), 3);
        assert_eq!(hamt.get(&key1), Some(&100));
        assert_eq!(hamt.get(&key2), Some(&200));
        assert_eq!(hamt.get(&key3), Some(&300));
    }

    #[test]
    fn collision_iterate_collision_bucket() {
        // Iteration should yield all entries from collision buckets
        let key1 = CollisionKey::new(1, 0xFFFF_0000);
        let key2 = CollisionKey::new(2, 0xFFFF_0000);

        let hamt: Hamt<CollisionKey, &str> = Hamt::new();
        let hamt = hamt.insert(key1.clone(), "first");
        let hamt = hamt.insert(key2.clone(), "second");

        let entries: Vec<_> = hamt.iter().collect();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn collision_preserves_original_on_insert() {
        // Structural sharing should work correctly with collisions
        let key1 = CollisionKey::new(1, 0xBEEF_CAFE);
        let key2 = CollisionKey::new(2, 0xBEEF_CAFE);

        let h1: Hamt<CollisionKey, &str> = Hamt::new();
        let h1 = h1.insert(key1.clone(), "first");
        let h2 = h1.insert(key2.clone(), "second");

        // h1 should still have only one entry
        assert_eq!(h1.len(), 1);
        assert_eq!(h1.get(&key2), None);

        // h2 should have both
        assert_eq!(h2.len(), 2);
        assert_eq!(h2.get(&key1), Some(&"first"));
        assert_eq!(h2.get(&key2), Some(&"second"));
    }
}
