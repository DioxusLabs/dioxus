use std::{
    any::Any,
    cell::{Cell, RefCell},
};

use bumpalo::Bump;

use crate::{any_props::AnyProps, arena::ElementId, bump_frame::BumpFrame, nodes::VTemplate};

pub type ScopeId = usize;

pub struct ScopeState {
    pub render_cnt: usize,

    pub node_arena_1: BumpFrame,
    pub node_arena_2: BumpFrame,

    pub parent: Option<*mut ScopeState>,
    pub container: ElementId,
    pub our_arena_idx: ScopeId,

    pub height: u32,
    pub fn_ptr: ComponentPtr,

    pub hook_arena: Bump,
    pub hook_vals: RefCell<Vec<*mut dyn Any>>,
    pub hook_idx: Cell<usize>,

    pub props: Box<dyn AnyProps>,
}

impl ScopeState {
    pub fn current_arena(&self) -> &BumpFrame {
        match self.render_cnt % 2 {
            0 => &self.node_arena_1,
            1 => &self.node_arena_2,
            _ => unreachable!(),
        }
    }
}

pub(crate) type ComponentPtr = *mut std::os::raw::c_void;

pub struct Scope<T = ()> {
    pub props: T,
    pub state: Cell<*const ScopeState>,
}

impl<T> std::ops::Deref for Scope<T> {
    type Target = ScopeState;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.state.get() }
    }
}
