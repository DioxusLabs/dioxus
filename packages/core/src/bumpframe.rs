use crate::hooklist::HookList;
use crate::{arena::ScopeArena, innerlude::*};
use appendlist::AppendList;
use bumpalo::Bump;
use futures::FutureExt;
use slotmap::DefaultKey;
use slotmap::SlotMap;
use std::marker::PhantomData;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    future::Future,
    ops::Deref,
    pin::Pin,
    rc::{Rc, Weak},
};
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

    pub fn from_frames(a: BumpFrame, b: BumpFrame) -> Self {
        Self {
            generation: 0.into(),
            frames: [a, b],
        }
    }

    pub fn cur_frame(&self) -> &BumpFrame {
        match *self.generation.borrow() & 1 == 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }
    pub fn cur_frame_mut(&mut self) -> &mut BumpFrame {
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

    pub fn next(&mut self) -> &mut BumpFrame {
        *self.generation.borrow_mut() += 1;

        if *self.generation.borrow() % 2 == 0 {
            &mut self.frames[0]
        } else {
            &mut self.frames[1]
        }
    }
}
