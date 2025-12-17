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

/// Number of bits per trie level (32-way branching).
const BITS: u32 = 5;

/// Branching factor (2^5 = 32).
const WIDTH: usize = 1_usize << BITS;

/// Mask for extracting index within a node.
const MASK: usize = WIDTH.saturating_sub(1);

/// A node in the persistent vector trie.
///
/// Nodes are either branch nodes (containing child nodes) or leaf nodes
/// (containing actual values). The structure is determined by the level
/// in the trie: level 0 nodes are leaves, all others are branches.
enum Node<T> {
    /// A branch node containing up to 32 child nodes.
    Branch(Rc<[Option<Rc<Self>>; WIDTH]>),
    /// A leaf node containing up to 32 values.
    Leaf(Rc<[T]>),
}

impl<T: Clone> Clone for Node<T> {
    #[inline]
    fn clone(&self) -> Self {
        match *self {
            Self::Branch(ref children) => Self::Branch(Rc::clone(children)),
            Self::Leaf(ref values) => Self::Leaf(Rc::clone(values)),
        }
    }
}

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
        self.get_from_root(index)
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
            .map(|node| Rc::new(Self::do_assoc(self.shift, node, index, value)));

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

    /// Gets an element from the root trie (not the tail).
    ///
    /// Navigates through the trie using index bits at each level until
    /// reaching a leaf node, then returns the element at the appropriate
    /// position within the leaf.
    fn get_from_root(&self, index: usize) -> Option<&T> {
        let mut node = self.root.as_ref()?;
        let mut level = self.shift;

        loop {
            match **node {
                Node::Branch(ref children) => {
                    let child_index = (index >> level) & MASK;
                    node = children.get(child_index)?.as_ref()?;
                    level = level.saturating_sub(BITS);
                }
                Node::Leaf(ref values) => {
                    let value_index = index & MASK;
                    return values.get(value_index);
                }
            }
        }
    }

    /// Recursively creates a new path with the updated value.
    ///
    /// Implements path copying for persistent updates. Only the nodes along
    /// the path from root to the updated element are copied; all other nodes
    /// are shared with the original trie.
    fn do_assoc(level: u32, node: &Rc<Node<T>>, index: usize, value: T) -> Node<T> {
        if level == 0 {
            // At leaf level
            if let Node::Leaf(ref values) = **node {
                let mut new_values: Vec<T> = values.iter().cloned().collect();
                let leaf_index = index & MASK;
                if let Some(slot) = new_values.get_mut(leaf_index) {
                    *slot = value;
                }
                return Node::Leaf(Rc::from(new_values));
            }
            // Should not happen in a well-formed trie
            return Node::Leaf(Rc::from([]));
        }

        // At branch level
        if let Node::Branch(ref children) = **node {
            let child_index = (index >> level) & MASK;
            let mut new_children: [Option<Rc<Node<T>>>; WIDTH] = (**children).clone();
            if let Some(child) = new_children.get_mut(child_index)
                && let Some(child_node) = child.as_ref()
            {
                let new_child =
                    Self::do_assoc(level.saturating_sub(BITS), child_node, index, value);
                *child = Some(Rc::new(new_child));
            }
            return Node::Branch(Rc::new(new_children));
        }

        // Should not happen in a well-formed trie
        Node::Branch(Rc::new(core::array::from_fn(|_| None)))
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
                    let new_root = Self::new_path(self.shift, tail_node);
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
                    let new_root = Self::do_push_tail(self.shift, root, tail_node, tail_offset);
                    (Rc::new(new_root), self.shift)
                }
            }
        }
    }

    /// Creates a new path from root to leaf for the given tail node.
    ///
    /// Used when growing the trie to create a new branch path down to
    /// the leaf level. Each intermediate level gets a branch with a
    /// single child at index 0.
    fn new_path(level: u32, node: Node<T>) -> Node<T> {
        if level == 0 {
            return node;
        }

        let subpath = Self::new_path(level.saturating_sub(BITS), node);
        let mut children: [Option<Rc<Node<T>>>; WIDTH] = core::array::from_fn(|_| None);
        if let Some(slot) = children.get_mut(0) {
            *slot = Some(Rc::new(subpath));
        }
        Node::Branch(Rc::new(children))
    }

    /// Recursively inserts the tail node at the correct position.
    ///
    /// Navigates through the trie using the index bits and inserts the
    /// tail node at the appropriate leaf position. Creates new branch
    /// nodes as needed for previously empty slots.
    fn do_push_tail(level: u32, node: &Rc<Node<T>>, tail_node: Node<T>, index: usize) -> Node<T> {
        if level == BITS {
            // Insert at this level
            if let Node::Branch(ref children) = **node {
                let mut new_children: [Option<Rc<Node<T>>>; WIDTH] = (**children).clone();
                let child_index = (index >> BITS) & MASK;
                if let Some(slot) = new_children.get_mut(child_index) {
                    *slot = Some(Rc::new(tail_node));
                }
                return Node::Branch(Rc::new(new_children));
            }
        }

        // Continue down the tree
        if let Node::Branch(ref children) = **node {
            let child_index = (index >> level) & MASK;
            let mut new_children: [Option<Rc<Node<T>>>; WIDTH] = (**children).clone();

            let new_child = match children.get(child_index) {
                Some(&Some(ref child)) => {
                    Self::do_push_tail(level.saturating_sub(BITS), child, tail_node, index)
                }
                _ => Self::new_path(level.saturating_sub(BITS), tail_node),
            };

            if let Some(slot) = new_children.get_mut(child_index) {
                *slot = Some(Rc::new(new_child));
            }
            return Node::Branch(Rc::new(new_children));
        }

        // Should not happen
        Node::Branch(Rc::new(core::array::from_fn(|_| None)))
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
        let new_tail = self.get_leaf_at(new_tail_offset);

        // Remove the rightmost leaf from the trie
        let (new_root, new_shift) = Self::do_pop_tail(self.shift, root, new_tail_offset);

        (new_tail, new_root, new_shift)
    }

    /// Gets the leaf node at the given index as a vector.
    fn get_leaf_at(&self, index: usize) -> Vec<T> {
        let Some(ref root_node) = self.root else {
            return Vec::new();
        };
        let mut node = root_node;
        let mut level = self.shift;

        loop {
            match **node {
                Node::Branch(ref children) => {
                    let child_index = (index >> level) & MASK;
                    let Some(&Some(ref child)) = children.get(child_index) else {
                        return Vec::new();
                    };
                    node = child;
                    level = level.saturating_sub(BITS);
                }
                Node::Leaf(ref values) => {
                    return values.iter().cloned().collect();
                }
            }
        }
    }

    /// Recursively removes the rightmost leaf from the trie.
    ///
    /// Returns the new root (None if trie becomes empty) and the new shift.
    fn do_pop_tail(level: u32, node: &Rc<Node<T>>, index: usize) -> (Option<Rc<Node<T>>>, u32) {
        if level == BITS {
            // At the leaf level, remove the child
            if let Node::Branch(ref children) = **node {
                let child_index = (index >> BITS) & MASK;
                let mut new_children: [Option<Rc<Node<T>>>; WIDTH] = (**children).clone();
                if let Some(slot) = new_children.get_mut(child_index) {
                    *slot = None;
                }

                // Check if only one child remains at index 0
                let non_empty_count = new_children.iter().filter(|child| child.is_some()).count();
                if non_empty_count == 0 {
                    return (None, BITS);
                }

                return (Some(Rc::new(Node::Branch(Rc::new(new_children)))), level);
            }
            return (None, BITS);
        }

        // Continue down the tree
        if let Node::Branch(ref children) = **node {
            let child_index = (index >> level) & MASK;

            let Some(&Some(ref child)) = children.get(child_index) else {
                return (Some(Rc::clone(node)), level);
            };

            let (new_child, _) = Self::do_pop_tail(level.saturating_sub(BITS), child, index);

            let mut new_children: [Option<Rc<Node<T>>>; WIDTH] = (**children).clone();
            if let Some(slot) = new_children.get_mut(child_index) {
                *slot = new_child;
            }

            // Check if we should shrink the tree
            let non_empty_count = new_children.iter().filter(|child| child.is_some()).count();
            if non_empty_count == 0 {
                return (None, BITS);
            }

            return (Some(Rc::new(Node::Branch(Rc::new(new_children)))), level);
        }

        (None, BITS)
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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Construction Tests
    // =========================================================================

    #[test]
    fn new_creates_empty_vec() {
        let pvec: PersistentVec<i32> = PersistentVec::new();
        assert!(pvec.is_empty());
        assert_eq!(pvec.len(), 0);
    }

    #[test]
    fn from_vec_empty() {
        let pvec: PersistentVec<i32> = PersistentVec::from_vec(Vec::new());
        assert!(pvec.is_empty());
    }

    #[test]
    fn from_vec_elements() {
        let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
        assert_eq!(pvec.len(), 3);
        assert_eq!(pvec.get(0), Some(&1));
        assert_eq!(pvec.get(1), Some(&2));
        assert_eq!(pvec.get(2), Some(&3));
    }

    // =========================================================================
    // Push Tests
    // =========================================================================

    #[test]
    fn push_single_element() {
        let pvec: PersistentVec<i32> = PersistentVec::new();
        let pvec = pvec.push(42);
        assert_eq!(pvec.len(), 1);
        assert_eq!(pvec.get(0), Some(&42));
    }

    #[test]
    fn push_multiple_elements() {
        let mut pvec: PersistentVec<i32> = PersistentVec::new();
        for i in 0..10 {
            pvec = pvec.push(i);
        }
        assert_eq!(pvec.len(), 10);
        for i in 0..10 {
            assert_eq!(pvec.get(i), Some(&(i as i32)));
        }
    }

    #[test]
    fn push_fills_tail_then_trie() {
        let mut pvec: PersistentVec<i32> = PersistentVec::new();
        // Push more than WIDTH elements to force trie creation
        for i in 0..50 {
            pvec = pvec.push(i);
        }
        assert_eq!(pvec.len(), 50);
        for i in 0..50 {
            assert_eq!(pvec.get(i), Some(&(i as i32)));
        }
    }

    #[test]
    fn push_large_vector() {
        let mut pvec: PersistentVec<i32> = PersistentVec::new();
        let count: i32 = 1000;
        for i in 0..count {
            pvec = pvec.push(i);
        }
        assert_eq!(pvec.len(), count as usize);
        for i in 0..count {
            assert_eq!(pvec.get(i as usize), Some(&i));
        }
    }

    // =========================================================================
    // Get Tests
    // =========================================================================

    #[test]
    fn get_out_of_bounds() {
        let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
        assert_eq!(pvec.get(5), None);
    }

    #[test]
    fn get_empty() {
        let pvec: PersistentVec<i32> = PersistentVec::new();
        assert_eq!(pvec.get(0), None);
    }

    // =========================================================================
    // Assoc Tests
    // =========================================================================

    #[test]
    fn assoc_in_tail() {
        let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
        let new_pvec = pvec.assoc(1, 42).unwrap();

        // Original unchanged
        assert_eq!(pvec.get(1), Some(&2));
        // New has updated value
        assert_eq!(new_pvec.get(1), Some(&42));
        // Other elements unchanged
        assert_eq!(new_pvec.get(0), Some(&1));
        assert_eq!(new_pvec.get(2), Some(&3));
    }

    #[test]
    fn assoc_in_trie() {
        let mut pvec: PersistentVec<i32> = PersistentVec::new();
        for i in 0..50 {
            pvec = pvec.push(i);
        }

        let new_pvec = pvec.assoc(5, 999).unwrap();

        // Original unchanged
        assert_eq!(pvec.get(5), Some(&5));
        // New has updated value
        assert_eq!(new_pvec.get(5), Some(&999));
    }

    #[test]
    fn assoc_out_of_bounds() {
        let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
        assert!(pvec.assoc(10, 42).is_none());
    }

    // =========================================================================
    // Pop Tests
    // =========================================================================

    #[test]
    fn pop_empty() {
        let pvec: PersistentVec<i32> = PersistentVec::new();
        assert!(pvec.pop().is_none());
    }

    #[test]
    fn pop_single_element() {
        let pvec = PersistentVec::from_vec(alloc::vec![42]);
        let popped = pvec.pop().unwrap();

        assert!(popped.is_empty());
        // Original unchanged
        assert_eq!(pvec.len(), 1);
    }

    #[test]
    fn pop_multiple_elements_in_tail() {
        let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
        let popped = pvec.pop().unwrap();

        assert_eq!(popped.len(), 2);
        assert_eq!(popped.get(0), Some(&1));
        assert_eq!(popped.get(1), Some(&2));
        // Original unchanged
        assert_eq!(pvec.len(), 3);
    }

    #[test]
    fn pop_across_tail_boundary() {
        // Create a vector with more than 32 elements
        let mut pvec: PersistentVec<i32> = PersistentVec::new();
        for i in 0..35 {
            pvec = pvec.push(i);
        }

        // Pop should work correctly across the boundary
        let popped = pvec.pop().unwrap();
        assert_eq!(popped.len(), 34);
        for i in 0..34 {
            assert_eq!(popped.get(i), Some(&(i as i32)));
        }
    }

    #[test]
    fn pop_preserves_original() {
        let v1 = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
        let v2 = v1.pop().unwrap();

        // v1 unchanged
        assert_eq!(v1.len(), 3);
        assert_eq!(v1.get(2), Some(&3));

        // v2 has one less element
        assert_eq!(v2.len(), 2);
        assert_eq!(v2.get(2), None);
    }

    #[test]
    fn pop_then_push() {
        let v1 = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
        let v2 = v1.pop().unwrap();
        let v3 = v2.push(99);

        assert_eq!(v3.len(), 3);
        assert_eq!(v3.get(0), Some(&1));
        assert_eq!(v3.get(1), Some(&2));
        assert_eq!(v3.get(2), Some(&99));
    }

    // =========================================================================
    // Structural Sharing Tests
    // =========================================================================

    #[test]
    fn push_preserves_original() {
        let v1 = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
        let v2 = v1.push(4);

        // v1 unchanged
        assert_eq!(v1.len(), 3);
        assert_eq!(v1.get(0), Some(&1));
        assert_eq!(v1.get(1), Some(&2));
        assert_eq!(v1.get(2), Some(&3));

        // v2 has new element
        assert_eq!(v2.len(), 4);
        assert_eq!(v2.get(3), Some(&4));
    }

    #[test]
    fn assoc_preserves_original() {
        let v1 = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
        let v2 = v1.assoc(1, 42).unwrap();

        // v1 unchanged
        assert_eq!(v1.get(1), Some(&2));
        // v2 has updated value
        assert_eq!(v2.get(1), Some(&42));
    }

    // =========================================================================
    // Iterator Tests
    // =========================================================================

    #[test]
    fn iter_empty() {
        let pvec: PersistentVec<i32> = PersistentVec::new();
        let collected: Vec<_> = pvec.iter().collect();
        assert!(collected.is_empty());
    }

    #[test]
    fn iter_elements() {
        let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
        let collected: Vec<_> = pvec.iter().cloned().collect();
        assert_eq!(collected, alloc::vec![1, 2, 3]);
    }

    #[test]
    fn iter_large() {
        let mut pvec: PersistentVec<i32> = PersistentVec::new();
        for i in 0..100 {
            pvec = pvec.push(i);
        }
        let collected: Vec<_> = pvec.iter().cloned().collect();
        let expected: Vec<i32> = (0..100).collect();
        assert_eq!(collected, expected);
    }

    #[test]
    fn iter_size_hint() {
        let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3, 4, 5]);
        let iter = pvec.iter();
        assert_eq!(iter.size_hint(), (5, Some(5)));
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    #[test]
    fn clone_shares_structure() {
        let v1 = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
        let v2 = v1.clone();

        assert_eq!(v1.len(), v2.len());
        for i in 0..v1.len() {
            assert_eq!(v1.get(i), v2.get(i));
        }
    }

    // =========================================================================
    // Default Test
    // =========================================================================

    #[test]
    fn default_is_empty() {
        let pvec: PersistentVec<i32> = PersistentVec::default();
        assert!(pvec.is_empty());
    }
}
