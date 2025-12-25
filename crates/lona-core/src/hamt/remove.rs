// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Remove operations for the HAMT.
//!
//! Contains all the recursive operations for removing key-value pairs
//! from the trie, including restructuring after removal.

use alloc::rc::Rc;
use alloc::vec::Vec;

use super::node::{Child, Node, bitmap_to_index, index_at_depth};

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
