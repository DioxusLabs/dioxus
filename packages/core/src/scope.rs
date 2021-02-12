use crate::inner::*;
use crate::nodes::VNode;
use crate::{context::hooks::Hook, diff::diff};
use bumpalo::Bump;
use generational_arena::Index;
use std::{
    any::TypeId, borrow::Borrow, cell::RefCell, future::Future, marker::PhantomData,
    sync::atomic::AtomicUsize,
};

/// Every component in Dioxus is represented by a `Scope`.
///
/// Scopes contain the state for hooks, the component's props, and other lifecycle information.
///
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components.
/// The actual contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
pub struct Scope {
    // TODO @Jon
    // These hooks are actually references into the hook arena
    // These two could be combined with "OwningRef" to remove unsafe usage
    // could also use ourborous
    hooks: RefCell<Vec<*mut Hook>>,
    hook_arena: typed_arena::Arena<Hook>,

    // Map to the parent
    parent: Option<Index>,

    // todo, do better with the active frame stuff
    frames: [Bump; 2],

    // somehow build this vnode with a lifetime tied to self
    cur_node: *mut VNode<'static>,

    active_frame: u8,

    // IE Which listeners need to be woken up?
    listeners: Vec<Box<dyn Fn()>>,

    //
    props_type: TypeId,
    caller: *const i32,
}

impl Scope {
    // create a new scope from a function
    pub(crate) fn new<T: 'static>(f: FC<T>, parent: Option<Index>) -> Self {
        // Capture the props type
        let props_type = TypeId::of::<T>();
        let hook_arena = typed_arena::Arena::new();
        let hooks = RefCell::new(Vec::new());

        let caller = f as *const i32;

        let frames = [Bump::new(), Bump::new()];

        let listeners = Vec::new();

        let active_frame = 1;

        let new = frames[0].alloc(VNode::Text(VText::new("")));
        let cur_node = new as *mut _;

        Self {
            hook_arena,
            hooks,
            props_type,
            caller,
            active_frame,
            listeners,
            parent,
            frames,
            cur_node,
        }
    }

    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    ///
    /// Props is ?Sized because we borrow the props
    pub(crate) fn run<'a, P: Properties + ?Sized>(&self, props: &'a P) {
        let bump = &self.frames[0];

        let ctx = Context {
            scope: &*self,
            _p: PhantomData {},
            arena: &self.hook_arena,
            hooks: &self.hooks,
            idx: 0.into(),
            bump,
        };

        /*
        SAFETY ALERT

        This particular usage of transmute is outlined in its docs https://doc.rust-lang.org/std/mem/fn.transmute.html
        We hide the generic bound on the function item by casting it to raw pointer. When the function is actually called,
        we transmute the function back using the props as reference.

        we could do a better check to make sure that the TypeID is correct
        --
        This is safe because we check that the generic type matches before casting.
        */
        let caller = unsafe { std::mem::transmute::<*const i32, FC<P>>(self.caller) };
        let new_nodes = caller(ctx, props);
        let old_nodes: &mut VNode<'static> = unsafe { &mut *self.cur_node };

        // perform the diff, dumping into the change list
        crate::diff::diff(old_nodes, &new_nodes);
    }
}
