// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Node implementation for the persistent vector trie.
//!
//! This module contains the internal `Node` enum and recursive trie operations
//! used by `PersistentVec`.

use alloc::rc::Rc;
use alloc::vec::Vec;

use super::{BITS, MASK, WIDTH};

/// A node in the persistent vector trie.
///
/// Nodes are either branch nodes (containing child nodes) or leaf nodes
/// (containing actual values). The structure is determined by the level
/// in the trie: level 0 nodes are leaves, all others are branches.
pub(super) enum Node<T> {
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

impl<T: Clone> Node<T> {
    /// Gets an element from this node at the given index and level.
    ///
    /// Navigates through the trie using index bits at each level until
    /// reaching a leaf node, then returns the element at the appropriate
    /// position within the leaf.
    pub(super) fn get(&self, index: usize, mut level: u32) -> Option<&T> {
        let mut node = self;

        loop {
            match *node {
                Self::Branch(ref children) => {
                    let child_index = (index >> level) & MASK;
                    let child = children.get(child_index)?.as_ref()?;
                    node = child;
                    level = level.saturating_sub(BITS);
                }
                Self::Leaf(ref values) => {
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
    pub(super) fn do_assoc(level: u32, node: &Rc<Self>, index: usize, value: T) -> Self {
        if level == 0 {
            // At leaf level
            if let Self::Leaf(ref values) = **node {
                let mut new_values: Vec<T> = values.iter().cloned().collect();
                let leaf_index = index & MASK;
                if let Some(slot) = new_values.get_mut(leaf_index) {
                    *slot = value;
                }
                return Self::Leaf(Rc::from(new_values));
            }
            // Should not happen in a well-formed trie
            return Self::Leaf(Rc::from([]));
        }

        // At branch level
        if let Self::Branch(ref children) = **node {
            let child_index = (index >> level) & MASK;
            let mut new_children: [Option<Rc<Self>>; WIDTH] = (**children).clone();
            if let Some(child) = new_children.get_mut(child_index)
                && let Some(child_node) = child.as_ref()
            {
                let new_child =
                    Self::do_assoc(level.saturating_sub(BITS), child_node, index, value);
                *child = Some(Rc::new(new_child));
            }
            return Self::Branch(Rc::new(new_children));
        }

        // Should not happen in a well-formed trie
        Self::Branch(Rc::new(core::array::from_fn(|_| None)))
    }

    /// Creates a new path from root to leaf for the given tail node.
    ///
    /// Used when growing the trie to create a new branch path down to
    /// the leaf level. Each intermediate level gets a branch with a
    /// single child at index 0.
    pub(super) fn new_path(level: u32, node: Self) -> Self {
        if level == 0 {
            return node;
        }

        let subpath = Self::new_path(level.saturating_sub(BITS), node);
        let mut children: [Option<Rc<Self>>; WIDTH] = core::array::from_fn(|_| None);
        if let Some(slot) = children.get_mut(0) {
            *slot = Some(Rc::new(subpath));
        }
        Self::Branch(Rc::new(children))
    }

    /// Recursively inserts the tail node at the correct position.
    ///
    /// Navigates through the trie using the index bits and inserts the
    /// tail node at the appropriate leaf position. Creates new branch
    /// nodes as needed for previously empty slots.
    pub(super) fn do_push_tail(level: u32, node: &Rc<Self>, tail_node: Self, index: usize) -> Self {
        if level == BITS {
            // Insert at this level
            if let Self::Branch(ref children) = **node {
                let mut new_children: [Option<Rc<Self>>; WIDTH] = (**children).clone();
                let child_index = (index >> BITS) & MASK;
                if let Some(slot) = new_children.get_mut(child_index) {
                    *slot = Some(Rc::new(tail_node));
                }
                return Self::Branch(Rc::new(new_children));
            }
        }

        // Continue down the tree
        if let Self::Branch(ref children) = **node {
            let child_index = (index >> level) & MASK;
            let mut new_children: [Option<Rc<Self>>; WIDTH] = (**children).clone();

            let new_child = match children.get(child_index) {
                Some(&Some(ref child)) => {
                    Self::do_push_tail(level.saturating_sub(BITS), child, tail_node, index)
                }
                _ => Self::new_path(level.saturating_sub(BITS), tail_node),
            };

            if let Some(slot) = new_children.get_mut(child_index) {
                *slot = Some(Rc::new(new_child));
            }
            return Self::Branch(Rc::new(new_children));
        }

        // Should not happen
        Self::Branch(Rc::new(core::array::from_fn(|_| None)))
    }

    /// Gets the leaf node at the given index as a vector.
    pub(super) fn get_leaf_at(&self, index: usize, mut level: u32) -> Vec<T> {
        let mut node = self;

        loop {
            match *node {
                Self::Branch(ref children) => {
                    let child_index = (index >> level) & MASK;
                    let Some(&Some(ref child)) = children.get(child_index) else {
                        return Vec::new();
                    };
                    node = child;
                    level = level.saturating_sub(BITS);
                }
                Self::Leaf(ref values) => {
                    return values.iter().cloned().collect();
                }
            }
        }
    }

    /// Recursively removes the rightmost leaf from the trie.
    ///
    /// Returns the new root (None if trie becomes empty) and the new shift.
    pub(super) fn do_pop_tail(
        level: u32,
        node: &Rc<Self>,
        index: usize,
    ) -> (Option<Rc<Self>>, u32) {
        if level == BITS {
            // At the leaf level, remove the child
            if let Self::Branch(ref children) = **node {
                let child_index = (index >> BITS) & MASK;
                let mut new_children: [Option<Rc<Self>>; WIDTH] = (**children).clone();
                if let Some(slot) = new_children.get_mut(child_index) {
                    *slot = None;
                }

                // Check if only one child remains at index 0
                let non_empty_count = new_children.iter().filter(|child| child.is_some()).count();
                if non_empty_count == 0 {
                    return (None, BITS);
                }

                return (Some(Rc::new(Self::Branch(Rc::new(new_children)))), level);
            }
            return (None, BITS);
        }

        // Continue down the tree
        if let Self::Branch(ref children) = **node {
            let child_index = (index >> level) & MASK;

            let Some(&Some(ref child)) = children.get(child_index) else {
                return (Some(Rc::clone(node)), level);
            };

            let (new_child, _) = Self::do_pop_tail(level.saturating_sub(BITS), child, index);

            let mut new_children: [Option<Rc<Self>>; WIDTH] = (**children).clone();
            if let Some(slot) = new_children.get_mut(child_index) {
                *slot = new_child;
            }

            // Check if we should shrink the tree
            let non_empty_count = new_children.iter().filter(|child| child.is_some()).count();
            if non_empty_count == 0 {
                return (None, BITS);
            }

            return (Some(Rc::new(Self::Branch(Rc::new(new_children)))), level);
        }

        (None, BITS)
    }
}
