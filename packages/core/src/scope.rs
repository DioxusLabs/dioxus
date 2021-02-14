use crate::context::hooks::Hook;
use crate::innerlude::*;
use crate::nodes::VNode;
use bumpalo::Bump;
use generational_arena::Index;
use std::{
    any::TypeId, borrow::Borrow, cell::RefCell, future::Future, marker::PhantomData,
    sync::atomic::AtomicUsize, todo,
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
    pub hooks: RefCell<Vec<*mut Hook>>,
    pub hook_arena: typed_arena::Arena<Hook>,

    // Map to the parent
    pub parent: Option<Index>,

    // todo, do better with the active frame stuff
    pub frames: [Bump; 2],

    // somehow build this vnode with a lifetime tied to self
    // This root node has  "static" lifetime, but it's really not static.
    // It's goverened by the oldest of the two frames and is switched every time a new render occurs
    // Use this node as if it were static is unsafe, and needs to be fixed with ourborous or owning ref
    // ! do not copy this reference are things WILL break !
    pub root_node: *mut VNode<'static>,

    pub active_frame: ActiveFrame,

    // IE Which listeners need to be woken up?
    pub listeners: Vec<Box<dyn Fn()>>,

    //
    pub props_type: TypeId,
    pub caller: *const i32,
}

pub enum ActiveFrame {
    First,
    Second,
}

impl ActiveFrame {
    fn next(&mut self) {
        match self {
            ActiveFrame::First => *self = ActiveFrame::Second,
            ActiveFrame::Second => *self = ActiveFrame::First,
        }
    }
}

impl Scope {
    // create a new scope from a function
    pub(crate) fn new<T: 'static>(f: FC<T>, parent: Option<Index>) -> Self {
        // Capture the props type
        let props_type = TypeId::of::<T>();
        let hook_arena = typed_arena::Arena::new();
        let hooks = RefCell::new(Vec::new());

        // Capture the caller
        let caller = f as *const i32;

        // Create the two buffers the componetn will render into
        // There will always be an "old" and "new"
        let frames = [Bump::new(), Bump::new()];

        let listeners = Vec::new();

        let active_frame = ActiveFrame::First;

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
            root_node: cur_node,
        }
    }

    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    ///
    /// Props is ?Sized because we borrow the props and don't need to know the size. P (sized) is used as a marker (unsized)
    pub(crate) fn run<'a, P: Properties + ?Sized>(&self, props: &'a P) {
        let bump = match self.active_frame {
            // If the active frame is the first, then we need to bump into the second
            ActiveFrame::First => &self.frames[1],
            // If the active frame is the second, then we need to bump into the first
            ActiveFrame::Second => &self.frames[0],
        }; // n.b, there might be a better way of doing this active frame stuff - perhaps swapping

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

        we could do a better check to make sure that the TypeID is correct before casting
        --
        This is safe because we check that the generic type matches before casting.
        */
        let caller = unsafe { std::mem::transmute::<*const i32, FC<P>>(self.caller) };
        let new_nodes = caller(ctx, props);
        let old_nodes: &mut VNode<'static> = unsafe { &mut *self.root_node };

        // TODO: Iterate through the new nodes
        // move any listeners into ourself

        // perform the diff, dumping into the mutable change list
        // this doesnt perform any "diff compression" where an event and a re-render
        // crate::diff::diff(old_nodes, &new_nodes);
        todo!()
    }
}
