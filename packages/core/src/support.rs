pub use crate::scope::*;
use crate::{arena::ScopeArena, innerlude::*};
use bumpalo::Bump;
use generational_arena::Arena;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    future::Future,
    ops::Deref,
    pin::Pin,
    rc::{Rc, Weak},
};
// We actually allocate the properties for components in their parent's properties
// We then expose a handle to use those props for render in the form of "OpaqueComponent"
pub(crate) type OpaqueComponent<'e> = dyn Fn(&'e Scope) -> VNode<'e> + 'e;
// pub(crate) type OpaqueComponent<'e> = dyn for<'b> Fn(&'b Scope) -> VNode<'b> + 'e;

#[derive(PartialEq, Debug, Clone, Default)]
pub(crate) struct EventQueue(pub Rc<RefCell<Vec<HeightMarker>>>);

impl EventQueue {
    pub fn new_channel(&self, height: u32, idx: ScopeIdx) -> Rc<dyn Fn()> {
        let inner = self.clone();
        let marker = HeightMarker { height, idx };
        Rc::new(move || inner.0.as_ref().borrow_mut().push(marker))
    }
}

/// A helper type that lets scopes be ordered by their height
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeightMarker {
    pub idx: ScopeIdx,
    pub height: u32,
}

impl Ord for HeightMarker {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.height.cmp(&other.height)
    }
}

impl PartialOrd for HeightMarker {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// NodeCtx is used to build VNodes in the component's memory space.
// This struct adds metadata to the final VNode about listeners, attributes, and children
#[derive(Clone)]
pub struct NodeCtx<'a> {
    pub scope_ref: &'a Scope,
    pub listener_id: RefCell<usize>,
}

impl<'a> NodeCtx<'a> {
    pub fn bump(&self) -> &'a Bump {
        &self.scope_ref.cur_frame().bump
    }
}

impl Debug for NodeCtx<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

#[derive(Debug, PartialEq, Hash)]
pub struct ContextId {
    // Which component is the scope in
    original: ScopeIdx,

    // What's the height of the scope
    height: u32,

    // Which scope is it (in order)
    id: u32,
}

pub struct ActiveFrame {
    // We use a "generation" for users of contents in the bump frames to ensure their data isn't broken
    pub generation: RefCell<usize>,

    // The double-buffering situation that we will use
    pub frames: [BumpFrame; 2],
}

pub struct BumpFrame {
    pub bump: Bump,
    pub head_node: VNode<'static>,
}

impl ActiveFrame {
    pub fn new() -> Self {
        Self::from_frames(
            BumpFrame {
                bump: Bump::new(),
                head_node: VNode::text(""),
            },
            BumpFrame {
                bump: Bump::new(),
                head_node: VNode::text(""),
            },
        )
    }

    pub(crate) fn from_frames(a: BumpFrame, b: BumpFrame) -> Self {
        Self {
            generation: 0.into(),
            frames: [a, b],
        }
    }

    pub(crate) fn cur_frame(&self) -> &BumpFrame {
        match *self.generation.borrow() & 1 == 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }
    pub(crate) fn cur_frame_mut(&mut self) -> &mut BumpFrame {
        match *self.generation.borrow() & 1 == 0 {
            true => &mut self.frames[0],
            false => &mut self.frames[1],
        }
    }

    pub fn current_head_node<'b>(&'b self) -> &'b VNode<'b> {
        let raw_node = match *self.generation.borrow() & 1 == 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        };

        // Give out our self-referential item with our own borrowed lifetime
        unsafe {
            let unsafe_head = &raw_node.head_node;
            let safe_node = std::mem::transmute::<&VNode<'static>, &VNode<'b>>(unsafe_head);
            safe_node
        }
    }

    pub fn prev_head_node<'b>(&'b self) -> &'b VNode<'b> {
        let raw_node = match *self.generation.borrow() & 1 != 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        };

        // Give out our self-referential item with our own borrowed lifetime
        unsafe {
            let unsafe_head = &raw_node.head_node;
            let safe_node = std::mem::transmute::<&VNode<'static>, &VNode<'b>>(unsafe_head);
            safe_node
        }
    }

    pub(crate) fn next(&mut self) -> &mut BumpFrame {
        *self.generation.borrow_mut() += 1;

        if *self.generation.borrow() % 2 == 0 {
            &mut self.frames[0]
        } else {
            &mut self.frames[1]
        }
    }
}
