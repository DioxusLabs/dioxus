//! Changelist
//! ----------
//!
//! This module exposes the "changelist" object which allows 3rd party implementors to handle diffs to the virtual dom.
//!
//! # Design
//! ---
//! In essence, the changelist object connects a diff of two vdoms to the actual edits required to update the output renderer.
//!
//! This abstraction relies on the assumption that the final renderer accepts a tree of elements. For most target platforms,
//! this is an appropriate abstraction .
//!
//! During the diff phase, the change list is built. Once the diff phase is over, the change list is finished and returned back
//! to the renderer. The renderer is responsible for propogating the updates to the final display.
//!
//! Because the change list references data internal to the vdom, it needs to be consumed by the renderer before the vdom
//! can continue to work. This means once a change list is generated, it should be consumed as fast as possible, otherwise the
//! dom will be blocked from progressing. This is enforced by lifetimes on the returend changelist object.
//!
//! # Known Issues
//! ----
//! - stack machine approach does not work when 3rd party extensions inject elements (breaking our view of the dom)

use std::cell::Ref;

use crate::innerlude::ScopeIdx;

pub type EditList<'src> = Vec<Edit<'src>>;
// pub struct EditList<'src> {
//     edits: Vec<Edit<'src>>,
// }

/// The `Edit` represents a single modifcation of the renderer tree.
/// todo @jon, go through and make certain fields static. tag names should be known at compile time
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
#[derive(Debug)]
pub enum Edit<'src_bump> {
    // ========================================================
    // Common Ops: The most common operation types
    // ========================================================
    SetText {
        text: &'src_bump str,
    },
    SetClass {
        class_name: &'src_bump str,
    },
    CreateTextNode {
        text: &'src_bump str,
    },
    CreateElement {
        // todo - make static?
        tag_name: &'src_bump str,
    },
    CreateElementNs {
        // todo - make static?
        tag_name: &'src_bump str,
        // todo - make static?
        ns: &'src_bump str,
    },

    // ========================================================
    // Attributes
    // ========================================================
    SetAttribute {
        name: &'src_bump str,
        value: &'src_bump str,
    },
    RemoveAttribute {
        name: &'src_bump str,
    },
    RemoveChild {
        n: u32,
    },

    // ============================================================
    // Event Listeners: Event types and IDs used to update the VDOM
    // ============================================================
    NewListener {
        // todo - make static?
        event: &'src_bump str,
        scope: ScopeIdx,
        id: usize,
    },
    UpdateListener {
        // todo - make static?
        event: &'src_bump str,
        scope: ScopeIdx,
        id: usize,
    },
    RemoveListener {
        // todo - make static?
        event: &'src_bump str,
    },

    // ========================================================
    // Cached Roots: The mount point for individual components
    //               Allows quick traversal to cached entrypoints
    // ========================================================
    // push a known node on to the stack
    TraverseToKnown {
        node: u32,
        // node: ScopeIdx,
    },
    // Add the current top of the stack to the known nodes
    MakeKnown {
        node: u32,
        // node: ScopeIdx,
    },
    // Remove the current top of the stack from the known nodes
    RemoveKnown,

    // ========================================================
    // Stack OPs: Operations for manipulating the stack machine
    // ========================================================
    PushReverseChild {
        n: u32,
    },
    PopPushChild {
        n: u32,
    },
    Pop,
    AppendChild,
    RemoveSelfAndNextSiblings {},
    ReplaceWith,
    SaveChildrenToTemporaries {
        temp: u32,
        start: u32,
        end: u32,
    },
    PushChild {
        n: u32,
    },
    PushTemporary {
        temp: u32,
    },
    InsertBefore,
    PopPushReverseChild {
        n: u32,
    },
}

/// The edit machine represents a stream of differences between two component trees.
///
/// This struct is interesting in that it keeps track of differences by borrowing
/// from the source rather than writing to a new buffer. This means that the virtual dom
/// *cannot* be updated while this machine is in existence without "unsafe".
///
/// This unsafety is handled by methods on the virtual dom and is not exposed via lib code.
pub struct EditMachine<'lock> {
    pub traversal: Traversal,
    next_temporary: u32,
    forcing_new_listeners: bool,

    // // if the current node is a "known" node
    // // any actions that modify this node should update the mapping
    // current_known: Option<u32>,
    pub emitter: EditList<'lock>,
}

impl<'lock> EditMachine<'lock> {
    pub fn new() -> Self {
        Self {
            // current_known: None,
            traversal: Traversal::new(),
            next_temporary: 0,
            forcing_new_listeners: false,
            emitter: EditList::<'lock>::default(),
        }
    }
}

// ===================================
//  Traversal Methods
// ===================================
impl<'src> EditMachine<'src> {
    pub fn go_down(&mut self) {
        self.traversal.down();
    }

    pub fn go_down_to_child(&mut self, index: usize) {
        self.traversal.down();
        self.traversal.sibling(index);
    }

    pub fn go_down_to_reverse_child(&mut self, index: usize) {
        self.traversal.down();
        self.traversal.reverse_sibling(index);
    }

    pub fn go_up(&mut self) {
        self.traversal.up();
    }

    pub fn go_to_sibling(&mut self, index: usize) {
        self.traversal.sibling(index);
    }

    pub fn go_to_temp_sibling(&mut self, temp: u32) {
        self.traversal.up();
        self.traversal.down_to_temp(temp);
    }

    pub fn go_down_to_temp_child(&mut self, temp: u32) {
        self.traversal.down_to_temp(temp);
    }

    pub fn commit_traversal(&mut self) {
        if self.traversal.is_committed() {
            return;
        }

        for mv in self.traversal.commit() {
            match mv {
                MoveTo::Parent => self.emitter.push(Edit::Pop {}),
                MoveTo::Child(n) => self.emitter.push(Edit::PushChild { n }),
                MoveTo::ReverseChild(n) => self.emitter.push(Edit::PushReverseChild { n }),
                MoveTo::Sibling(n) => self.emitter.push(Edit::PopPushChild { n }),
                MoveTo::ReverseSibling(n) => self.emitter.push(Edit::PopPushReverseChild { n }),
                MoveTo::TempChild(temp) => self.emitter.push(Edit::PushTemporary { temp }),
            }
        }
    }

    pub fn traversal_is_committed(&self) -> bool {
        self.traversal.is_committed()
    }
}

// ===================================
//  Stack methods
// ===================================
impl<'a> EditMachine<'a> {
    pub fn next_temporary(&self) -> u32 {
        self.next_temporary
    }

    pub fn set_next_temporary(&mut self, next_temporary: u32) {
        self.next_temporary = next_temporary;
    }

    pub fn save_children_to_temporaries(&mut self, start: usize, end: usize) -> u32 {
        debug_assert!(self.traversal_is_committed());
        debug_assert!(start < end);
        let temp_base = self.next_temporary;
        self.next_temporary = temp_base + (end - start) as u32;
        self.emitter.push(Edit::SaveChildrenToTemporaries {
            temp: temp_base,
            start: start as u32,
            end: end as u32,
        });
        temp_base
    }

    pub fn push_temporary(&mut self, temp: u32) {
        debug_assert!(self.traversal_is_committed());
        // debug!("emit: push_temporary({})", temp);
        self.emitter.push(Edit::PushTemporary { temp });
        // self.emitter.push_temporary(temp);
    }

    pub fn remove_child(&mut self, child: usize) {
        debug_assert!(self.traversal_is_committed());
        // debug!("emit: remove_child({})", child);
        // self.emitter.remove_child(child as u32);
        self.emitter.push(Edit::RemoveChild { n: child as u32 })
    }

    pub fn insert_before(&mut self) {
        debug_assert!(self.traversal_is_committed());
        // debug!("emit: insert_before()");
        // self.emitter.insert_before();
        self.emitter.push(Edit::InsertBefore {})
    }

    pub fn set_text(&mut self, text: &'a str) {
        debug_assert!(self.traversal_is_committed());
        // debug!("emit: set_text({:?})", text);
        // self.emitter.set_text(text);
        self.emitter.push(Edit::SetText { text });
        // .set_text(text.as_ptr() as u32, text.len() as u32);
    }

    pub fn remove_self_and_next_siblings(&mut self) {
        debug_assert!(self.traversal_is_committed());
        // debug!("emit: remove_self_and_next_siblings()");
        self.emitter.push(Edit::RemoveSelfAndNextSiblings {});
        // self.emitter.remove_self_and_next_siblings();
    }

    pub fn replace_with(&mut self) {
        debug_assert!(self.traversal_is_committed());
        // debug!("emit: replace_with()");
        self.emitter.push(Edit::ReplaceWith {});
        // if let Some(id) = self.current_known {
        //     // update mapping
        //     self.emitter.push(Edit::MakeKnown{node: id});
        //     self.current_known = None;
        // }
        // self.emitter.replace_with();
    }

    pub fn set_attribute(&mut self, name: &'a str, value: &'a str, is_namespaced: bool) {
        debug_assert!(self.traversal_is_committed());
        // todo!()
        if name == "class" && !is_namespaced {
            // let class_id = self.ensure_string(value);
            // let class_id = self.ensure_string(value);
            // debug!("emit: set_class({:?})", value);
            // self.emitter.set_class(class_id.into());
            self.emitter.push(Edit::SetClass { class_name: value });
        } else {
            self.emitter.push(Edit::SetAttribute { name, value });
            // let name_id = self.ensure_string(name);
            // let value_id = self.ensure_string(value);
            // debug!("emit: set_attribute({:?}, {:?})", name, value);
            // self.state
            //     .emitter
            //     .set_attribute(name_id.into(), value_id.into());
        }
    }

    pub fn remove_attribute(&mut self, name: &'a str) {
        // todo!("figure out how to get this working with ensure string");
        self.emitter.push(Edit::RemoveAttribute { name });
        // self.emitter.remove_attribute(name);
        // debug_assert!(self.traversal_is_committed());
        // // debug!("emit: remove_attribute({:?})", name);
        // let name_id = self.ensure_string(name);
        // self.emitter.remove_attribute(name_id.into());
    }

    pub fn append_child(&mut self) {
        debug_assert!(self.traversal_is_committed());
        // debug!("emit: append_child()");
        self.emitter.push(Edit::AppendChild {});
        // self.emitter.append_child();
    }

    pub fn create_text_node(&mut self, text: &'a str) {
        debug_assert!(self.traversal_is_committed());
        self.emitter.push(Edit::CreateTextNode { text });
    }

    pub fn create_element(&mut self, tag_name: &'a str) {
        debug_assert!(self.traversal_is_committed());
        self.emitter.push(Edit::CreateElement { tag_name });
    }

    pub fn create_element_ns(&mut self, tag_name: &'a str, ns: &'a str) {
        debug_assert!(self.traversal_is_committed());
        self.emitter.push(Edit::CreateElementNs { tag_name, ns });
    }

    pub fn push_force_new_listeners(&mut self) -> bool {
        let old = self.forcing_new_listeners;
        self.forcing_new_listeners = true;
        old
    }

    pub fn pop_force_new_listeners(&mut self, previous: bool) {
        debug_assert!(self.forcing_new_listeners);
        self.forcing_new_listeners = previous;
    }

    pub fn new_event_listener(&mut self, event: &'a str, scope: ScopeIdx, id: usize) {
        debug_assert!(self.traversal_is_committed());
        self.emitter.push(Edit::NewListener { event, scope, id });
    }

    pub fn update_event_listener(&mut self, event: &'a str, scope: ScopeIdx, id: usize) {
        debug_assert!(self.traversal_is_committed());
        if self.forcing_new_listeners {
            self.new_event_listener(event, scope, id);
            return;
        }

        self.emitter.push(Edit::NewListener { event, scope, id });
    }

    pub fn remove_event_listener(&mut self, event: &'a str) {
        debug_assert!(self.traversal_is_committed());
        self.emitter.push(Edit::RemoveListener { event });
        // debug!("emit: remove_event_listener({:?})", event);
    }

    pub fn save_known_root(&mut self, id: u32) {
        log::debug!("emit: save_known_root({:?})", id);
        self.emitter.push(Edit::MakeKnown { node: id })
    }

    pub fn load_known_root(&mut self, id: u32) {
        log::debug!("emit: TraverseToKnown({:?})", id);
        // self.current_known = Some(id);
        self.emitter.push(Edit::TraverseToKnown { node: id })
    }
}

// Keeps track of where we are moving in a DOM tree, and shortens traversal
// paths between mutations to their minimal number of operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveTo {
    /// Move from the current node up to its parent.
    Parent,

    /// Move to the current node's n^th child.
    Child(u32),

    /// Move to the current node's n^th from last child.
    ReverseChild(u32),

    /// Move to the n^th sibling. Not relative from the current
    /// location. Absolute indexed within all of the current siblings.
    Sibling(u32),

    /// Move to the n^th from last sibling. Not relative from the current
    /// location. Absolute indexed within all of the current siblings.
    ReverseSibling(u32),

    /// Move down to the given saved temporary child.
    TempChild(u32),
}

#[derive(Debug)]
pub struct Traversal {
    uncommitted: Vec<MoveTo>,
}

impl Traversal {
    /// Construct a new `Traversal` with its internal storage backed by the
    /// given bump arena.
    pub fn new() -> Traversal {
        Traversal {
            uncommitted: Vec::with_capacity(32),
        }
    }

    /// Move the traversal up in the tree.
    pub fn up(&mut self) {
        match self.uncommitted.last() {
            Some(MoveTo::Sibling(_)) | Some(MoveTo::ReverseSibling(_)) => {
                self.uncommitted.pop();
                self.uncommitted.push(MoveTo::Parent);
            }
            Some(MoveTo::TempChild(_)) | Some(MoveTo::Child(_)) | Some(MoveTo::ReverseChild(_)) => {
                self.uncommitted.pop();
                // And we're back at the parent.
            }
            _ => {
                self.uncommitted.push(MoveTo::Parent);
            }
        }
    }

    /// Move the traversal down in the tree to the first child of the current
    /// node.
    pub fn down(&mut self) {
        if let Some(&MoveTo::Parent) = self.uncommitted.last() {
            self.uncommitted.pop();
            self.sibling(0);
        } else {
            self.uncommitted.push(MoveTo::Child(0));
        }
    }

    /// Move the traversal to the n^th sibling.
    pub fn sibling(&mut self, index: usize) {
        let index = index as u32;
        match self.uncommitted.last_mut() {
            Some(MoveTo::Sibling(ref mut n)) | Some(MoveTo::Child(ref mut n)) => {
                *n = index;
            }
            Some(MoveTo::ReverseSibling(_)) => {
                self.uncommitted.pop();
                self.uncommitted.push(MoveTo::Sibling(index));
            }
            Some(MoveTo::TempChild(_)) | Some(MoveTo::ReverseChild(_)) => {
                self.uncommitted.pop();
                self.uncommitted.push(MoveTo::Child(index))
            }
            _ => {
                self.uncommitted.push(MoveTo::Sibling(index));
            }
        }
    }

    /// Move the the n^th from last sibling.
    pub fn reverse_sibling(&mut self, index: usize) {
        let index = index as u32;
        match self.uncommitted.last_mut() {
            Some(MoveTo::ReverseSibling(ref mut n)) | Some(MoveTo::ReverseChild(ref mut n)) => {
                *n = index;
            }
            Some(MoveTo::Sibling(_)) => {
                self.uncommitted.pop();
                self.uncommitted.push(MoveTo::ReverseSibling(index));
            }
            Some(MoveTo::TempChild(_)) | Some(MoveTo::Child(_)) => {
                self.uncommitted.pop();
                self.uncommitted.push(MoveTo::ReverseChild(index))
            }
            _ => {
                self.uncommitted.push(MoveTo::ReverseSibling(index));
            }
        }
    }

    /// Go to the given saved temporary.
    pub fn down_to_temp(&mut self, temp: u32) {
        match self.uncommitted.last() {
            Some(MoveTo::Sibling(_)) | Some(MoveTo::ReverseSibling(_)) => {
                self.uncommitted.pop();
            }
            Some(MoveTo::Parent)
            | Some(MoveTo::TempChild(_))
            | Some(MoveTo::Child(_))
            | Some(MoveTo::ReverseChild(_))
            | None => {
                // Can't remove moves to parents since we rely on their stack
                // pops.
            }
        }
        self.uncommitted.push(MoveTo::TempChild(temp));
    }

    /// Are all the traversal's moves committed? That is, are there no moves
    /// that have *not* been committed yet?
    #[inline]
    pub fn is_committed(&self) -> bool {
        self.uncommitted.is_empty()
    }

    /// Commit this traversals moves and return the optimized path from the last
    /// commit.
    #[inline]
    pub fn commit(&mut self) -> std::vec::Drain<'_, MoveTo> {
        self.uncommitted.drain(..)
    }

    #[inline]
    pub fn reset(&mut self) {
        self.uncommitted.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traversal() {
        fn t<F>(f: F) -> Box<dyn FnMut(&mut Traversal)>
        where
            F: 'static + FnMut(&mut Traversal),
        {
            Box::new(f) as _
        }

        for (mut traverse, expected_moves) in vec![
            (
                t(|t| {
                    t.down();
                }),
                vec![MoveTo::Child(0)],
            ),
            (
                t(|t| {
                    t.up();
                }),
                vec![MoveTo::Parent],
            ),
            (
                t(|t| {
                    t.sibling(42);
                }),
                vec![MoveTo::Sibling(42)],
            ),
            (
                t(|t| {
                    t.down();
                    t.up();
                }),
                vec![],
            ),
            (
                t(|t| {
                    t.down();
                    t.sibling(2);
                    t.up();
                }),
                vec![],
            ),
            (
                t(|t| {
                    t.down();
                    t.sibling(3);
                }),
                vec![MoveTo::Child(3)],
            ),
            (
                t(|t| {
                    t.down();
                    t.sibling(4);
                    t.sibling(8);
                }),
                vec![MoveTo::Child(8)],
            ),
            (
                t(|t| {
                    t.sibling(1);
                    t.sibling(1);
                }),
                vec![MoveTo::Sibling(1)],
            ),
            (
                t(|t| {
                    t.reverse_sibling(3);
                }),
                vec![MoveTo::ReverseSibling(3)],
            ),
            (
                t(|t| {
                    t.down();
                    t.reverse_sibling(3);
                }),
                vec![MoveTo::ReverseChild(3)],
            ),
            (
                t(|t| {
                    t.down();
                    t.reverse_sibling(3);
                    t.up();
                }),
                vec![],
            ),
            (
                t(|t| {
                    t.down();
                    t.reverse_sibling(3);
                    t.reverse_sibling(6);
                }),
                vec![MoveTo::ReverseChild(6)],
            ),
            (
                t(|t| {
                    t.up();
                    t.reverse_sibling(3);
                    t.reverse_sibling(6);
                }),
                vec![MoveTo::Parent, MoveTo::ReverseSibling(6)],
            ),
            (
                t(|t| {
                    t.up();
                    t.sibling(3);
                    t.sibling(6);
                }),
                vec![MoveTo::Parent, MoveTo::Sibling(6)],
            ),
            (
                t(|t| {
                    t.sibling(3);
                    t.sibling(6);
                    t.up();
                }),
                vec![MoveTo::Parent],
            ),
            (
                t(|t| {
                    t.reverse_sibling(3);
                    t.reverse_sibling(6);
                    t.up();
                }),
                vec![MoveTo::Parent],
            ),
            (
                t(|t| {
                    t.down();
                    t.down_to_temp(3);
                }),
                vec![MoveTo::Child(0), MoveTo::TempChild(3)],
            ),
            (
                t(|t| {
                    t.down_to_temp(3);
                    t.sibling(5);
                }),
                vec![MoveTo::Child(5)],
            ),
            (
                t(|t| {
                    t.down_to_temp(3);
                    t.reverse_sibling(5);
                }),
                vec![MoveTo::ReverseChild(5)],
            ),
            (
                t(|t| {
                    t.down_to_temp(3);
                    t.up();
                }),
                vec![],
            ),
            (
                t(|t| {
                    t.sibling(2);
                    t.up();
                    t.down_to_temp(3);
                }),
                vec![MoveTo::Parent, MoveTo::TempChild(3)],
            ),
            (
                t(|t| {
                    t.up();
                    t.down_to_temp(3);
                }),
                vec![MoveTo::Parent, MoveTo::TempChild(3)],
            ),
        ] {
            let mut traversal = Traversal::new();
            traverse(&mut traversal);
            let actual_moves: Vec<_> = traversal.commit().collect();
            assert_eq!(actual_moves, expected_moves);
        }
    }
}
