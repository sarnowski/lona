// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Persistent vector trie implementation.
//!
//! This module provides a persistent (immutable) vector data structure using
//! a 32-way branching trie with tail optimization. Operations are O(log32 n),
//! which is effectively constant for practical collection sizes.
//!
//! The implementation uses structural sharing to enable efficient copy-on-write
//! semantics. When a vector is modified, only the path from root to the modified
//! element is copied; the rest of the structure is shared.
//!
//! This module is internal and not exported directly. Use [`crate::vector::Vector`]
//! for the public API.

use alloc::rc::Rc;
use alloc::vec::Vec;

use core::iter::FusedIterator;

mod node;

#[cfg(test)]
mod tests;

use node::Node;

/// Number of bits per trie level (32-way branching).
const BITS: u32 = 5;

/// Branching factor (2^5 = 32).
const WIDTH: usize = 1_usize << BITS;

/// Mask for extracting index within a node.
const MASK: usize = WIDTH.saturating_sub(1);

/// A persistent vector with O(log32 n) access and update operations.
///
/// Uses a 32-way branching trie with tail optimization for efficient appends.
/// The tail holds the last chunk of elements (up to 32) for fast push operations.
///
/// # Structural Sharing
///
/// When a vector is modified, only the nodes along the path to the modification
/// are copied. All other nodes are shared between the old and new vectors.
pub struct PersistentVec<T> {
    /// Number of elements in the vector.
    len: usize,
    /// Bit shift for the root level (depth * BITS).
    shift: u32,
    /// Root of the trie (None if all elements fit in the tail).
    root: Option<Rc<Node<T>>>,
    /// Tail array holding the last chunk of elements.
    tail: Rc<[T]>,
}

impl<T: Clone> Clone for PersistentVec<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            len: self.len,
            shift: self.shift,
            root: self.root.clone(),
            tail: Rc::clone(&self.tail),
        }
    }
}

impl<T: Clone> Default for PersistentVec<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> PersistentVec<T> {
    /// Creates an empty persistent vector.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            len: 0,
            shift: BITS,
            root: None,
            tail: Rc::from([]),
        }
    }

    /// Creates a persistent vector from a standard vector.
    #[must_use]
    pub fn from_vec(values: Vec<T>) -> Self {
        let mut pvec = Self::new();
        for value in values {
            pvec = pvec.push(value);
        }
        pvec
    }

    /// Returns the number of elements in the vector.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the vector is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns a reference to the element at the given index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }

        // Check if the index is in the tail
        let tail_offset = self.tail_offset();
        if index >= tail_offset {
            let tail_index = index.saturating_sub(tail_offset);
            return self.tail.get(tail_index);
        }

        // Navigate through the trie
        self.root.as_ref()?.get(index, self.shift)
    }

    /// Returns a new vector with the element at `index` replaced by `value`.
    #[must_use]
    pub fn assoc(&self, index: usize, value: T) -> Option<Self> {
        if index >= self.len {
            return None;
        }

        let tail_offset = self.tail_offset();
        if index >= tail_offset {
            // Update in tail
            let tail_index = index.saturating_sub(tail_offset);
            let mut new_tail: Vec<T> = self.tail.iter().cloned().collect();
            if let Some(slot) = new_tail.get_mut(tail_index) {
                *slot = value;
            }
            return Some(Self {
                len: self.len,
                shift: self.shift,
                root: self.root.clone(),
                tail: Rc::from(new_tail),
            });
        }

        // Update in trie
        let new_root = self
            .root
            .as_ref()
            .map(|node| Rc::new(Node::do_assoc(self.shift, node, index, value)));

        Some(Self {
            len: self.len,
            shift: self.shift,
            root: new_root,
            tail: Rc::clone(&self.tail),
        })
    }

    /// Returns a new vector with `value` appended to the end.
    #[must_use]
    pub fn push(&self, value: T) -> Self {
        // Check if there's room in the tail
        if self.tail.len() < WIDTH {
            let mut new_tail: Vec<T> = self.tail.iter().cloned().collect();
            new_tail.push(value);
            return Self {
                len: self.len.saturating_add(1),
                shift: self.shift,
                root: self.root.clone(),
                tail: Rc::from(new_tail),
            };
        }

        // Tail is full, need to push it into the trie
        let tail_node = Node::Leaf(Rc::clone(&self.tail));
        let (new_root, new_shift) = self.push_tail(tail_node);

        Self {
            len: self.len.saturating_add(1),
            shift: new_shift,
            root: Some(new_root),
            tail: Rc::from([value]),
        }
    }

    /// Returns a new vector with the last element removed.
    ///
    /// Returns `None` if the vector is empty. The removed element is not
    /// returned; use [`get`](Self::get) to retrieve it first if needed.
    ///
    /// This operation shares structure with the original vector.
    #[must_use]
    pub fn pop(&self) -> Option<Self> {
        if self.is_empty() {
            return None;
        }

        let new_len = self.len.saturating_sub(1);

        // If the tail has more than one element, just shrink it
        if self.tail.len() > 1 {
            let new_tail: Vec<T> = self
                .tail
                .iter()
                .take(self.tail.len().saturating_sub(1))
                .cloned()
                .collect();
            return Some(Self {
                len: new_len,
                shift: self.shift,
                root: self.root.clone(),
                tail: Rc::from(new_tail),
            });
        }

        // Tail has exactly one element, need to get new tail from trie
        if new_len == 0 {
            return Some(Self::new());
        }

        // Get the last leaf from the trie to become the new tail
        let tail_offset = new_len.saturating_sub(1) >> BITS << BITS;
        let (new_tail, new_root, new_shift) = self.pop_tail(tail_offset);

        Some(Self {
            len: new_len,
            shift: new_shift,
            root: new_root,
            tail: Rc::from(new_tail),
        })
    }

    /// Returns an iterator over references to the elements.
    #[inline]
    #[must_use]
    pub const fn iter(&self) -> Iter<'_, T> {
        Iter {
            vec: self,
            index: 0,
            end: self.len,
        }
    }

    /// Returns a slice of the underlying tail for efficient access.
    #[inline]
    #[must_use]
    pub fn tail_slice(&self) -> &[T] {
        &self.tail
    }

    /// Returns the index where the tail starts.
    ///
    /// The tail holds the last chunk of elements (up to 32) for O(1) push
    /// operations. This method computes where the trie ends and the tail begins.
    #[inline]
    const fn tail_offset(&self) -> usize {
        if self.len < WIDTH {
            0
        } else {
            // Round down to the nearest multiple of WIDTH
            self.len.saturating_sub(1) >> BITS << BITS
        }
    }

    /// Pushes the full tail into the trie and returns the new root.
    ///
    /// When the tail is full (32 elements), it's promoted into the trie.
    /// This may require growing the trie height if the current capacity
    /// is exhausted.
    fn push_tail(&self, tail_node: Node<T>) -> (Rc<Node<T>>, u32) {
        match self.root {
            None => {
                // First node in the trie - wrap the leaf in a branch
                let mut children: [Option<Rc<Node<T>>>; WIDTH] = core::array::from_fn(|_| None);
                if let Some(slot) = children.get_mut(0) {
                    *slot = Some(Rc::new(tail_node));
                }
                (Rc::new(Node::Branch(Rc::new(children))), BITS)
            }
            Some(ref root) => {
                // Check if we need to grow the tree
                let tail_offset = self.tail_offset();
                let capacity = 1_usize << (self.shift.saturating_add(BITS));

                if tail_offset >= capacity {
                    // Need to add a new level
                    let new_root = Node::new_path(self.shift, tail_node);
                    let mut children: [Option<Rc<Node<T>>>; WIDTH] = core::array::from_fn(|_| None);
                    if let Some(slot) = children.get_mut(0) {
                        *slot = Some(Rc::clone(root));
                    }
                    if let Some(slot) = children.get_mut(1) {
                        *slot = Some(Rc::new(new_root));
                    }
                    (
                        Rc::new(Node::Branch(Rc::new(children))),
                        self.shift.saturating_add(BITS),
                    )
                } else {
                    // Insert at the appropriate location
                    let new_root = Node::do_push_tail(self.shift, root, tail_node, tail_offset);
                    (Rc::new(new_root), self.shift)
                }
            }
        }
    }

    /// Pops the rightmost leaf from the trie to become the new tail.
    ///
    /// Returns the leaf values, the new root (if any), and the new shift.
    /// This may shrink the trie height if the root becomes a single-child branch.
    fn pop_tail(&self, new_tail_offset: usize) -> (Vec<T>, Option<Rc<Node<T>>>, u32) {
        let Some(ref root) = self.root else {
            return (Vec::new(), None, BITS);
        };

        // Get the leaf that will become the new tail
        let new_tail = root.get_leaf_at(new_tail_offset, self.shift);

        // Remove the rightmost leaf from the trie
        let (new_root, new_shift) = Node::do_pop_tail(self.shift, root, new_tail_offset);

        (new_tail, new_root, new_shift)
    }
}

/// Iterator over references to elements in a persistent vector.
pub struct Iter<'pvec, T> {
    vec: &'pvec PersistentVec<T>,
    index: usize,
    end: usize,
}

impl<'pvec, T: Clone> Iterator for Iter<'pvec, T> {
    type Item = &'pvec T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.end {
            return None;
        }
        let item = self.vec.get(self.index)?;
        self.index = self.index.saturating_add(1);
        Some(item)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.end.saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<T: Clone> ExactSizeIterator for Iter<'_, T> {}
impl<T: Clone> FusedIterator for Iter<'_, T> {}

impl<'pvec, T: Clone> IntoIterator for &'pvec PersistentVec<T> {
    type Item = &'pvec T;
    type IntoIter = Iter<'pvec, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
