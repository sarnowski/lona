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

mod iter;
mod node;

pub use iter::HamtIter;
use node::{Node, RemoveResult, get_rec, hash_key, index_at_depth, insert_rec, remove_rec};

#[cfg(test)]
mod tests;

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
                let leaf = node::Child::Leaf { hash, key, value };
                let children: Vec<node::Child<K, V>> = alloc::vec![leaf];
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
            .map_or_else(Vec::new, |node| alloc::vec![iter::IterState::Node(node)]);
        HamtIter::new(stack)
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
