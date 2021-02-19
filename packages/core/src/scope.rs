use crate::context::hooks::Hook;
use crate::innerlude::*;
use crate::nodes::VNode;
use bumpalo::Bump;
use generational_arena::Index;
use owning_ref::StableAddress;
use std::{
    any::TypeId,
    borrow::{Borrow, BorrowMut},
    cell::{RefCell, UnsafeCell},
    future::Future,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::AtomicUsize,
    todo,
};

/// Every component in Dioxus is represented by a `Scope`.
///
/// Scopes contain the state for hooks, the component's props, and other lifecycle information.
///
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components.
/// The actual contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
pub struct Scope {
    // pub(crate) struct Scope {
    // TODO @Jon
    // These hooks are actually references into the hook arena
    // These two could be combined with "OwningRef" to remove unsafe usage
    // could also use ourborous
    pub hooks: RefCell<Vec<*mut Hook>>,
    pub hook_arena: typed_arena::Arena<Hook>,

    // Map to the parent
    pub parent: Option<Index>,

    // todo, do better with the active frame stuff
    // somehow build this vnode with a lifetime tied to self
    // This root node has  "static" lifetime, but it's really not static.
    // It's goverened by the oldest of the two frames and is switched every time a new render occurs
    // Use this node as if it were static is unsafe, and needs to be fixed with ourborous or owning ref
    // ! do not copy this reference are things WILL break !
    pub frames: ActiveFrame,

    // IE Which listeners need to be woken up?
    pub listeners: Vec<Box<dyn Fn()>>,

    pub props: Box<dyn std::any::Any>,

    //
    pub props_type: TypeId,
    pub caller: *const i32,
}

impl Scope {
    // create a new scope from a function
    pub fn new<T: 'static>(
        // pub(crate) fn new<T: 'static>(
        f: FC<T>,
        props: impl Properties + 'static,
        parent: Option<Index>,
    ) -> Self {
        // Capture the props type
        let props_type = TypeId::of::<T>();
        let hook_arena = typed_arena::Arena::new();
        let hooks = RefCell::new(Vec::new());

        // Capture the caller
        let caller = f as *const i32;

        let listeners = Vec::new();

        let old_frame = BumpFrame {
            bump: Bump::new(),
            head_node: VNode::text(""),
        };

        let new_frame = BumpFrame {
            bump: Bump::new(),
            head_node: VNode::text(""),
        };

        let frames = ActiveFrame::from_frames(old_frame, new_frame);
        let props = Box::new(props);

        Self {
            hook_arena,
            hooks,
            props_type,
            caller,
            frames,
            listeners,
            parent,
            props,
        }
    }

    /// Update this component's props with a new set of props
    ///
    ///
    pub(crate) fn update_props<P: Properties + Sized + 'static>(
        &mut self,
        new_props: Box<P>,
    ) -> crate::error::Result<()> {
        Ok(())
    }

    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    ///
    /// Props is ?Sized because we borrow the props and don't need to know the size. P (sized) is used as a marker (unsized)
    pub fn run<'bump, P: Properties + Sized + 'static>(&'bump mut self) {
        let frame = {
            let frame = self.frames.next();
            frame.bump.reset();
            frame
        };

        let ctx: Context<'bump> = Context {
            arena: &self.hook_arena,
            hooks: &self.hooks,
            bump: &frame.bump,
            idx: 0.into(),
            _p: PhantomData {},
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
        let nodes: VNode<'bump> = caller(ctx, self.props.downcast_ref::<P>().unwrap());

        /*
        SAFETY ALERT

        DO NOT USE THIS VNODE WITHOUT THE APPOPRIATE ACCESSORS.
        KEEPING THIS STATIC REFERENCE CAN LEAD TO UB.

        Some things to note:
        - The VNode itself is bound to the lifetime, but it itself is owned by scope.
        - The VNode has a private API and can only be used from accessors.
        - Public API cannot drop or destructure VNode
        */

        let unsafe_node = unsafe { std::mem::transmute::<VNode<'bump>, VNode<'static>>(nodes) };
        frame.head_node = unsafe_node;
    }

    /// Accessor to get the root node and its children (safely)\
    /// Scope is self-referntial, so we are forced to use the 'static lifetime to cheat
    pub fn current_root_node<'bump>(&'bump self) -> &'bump VNode<'bump> {
        self.frames.current_head_node()
    }
    pub fn prev_root_node<'bump>(&'bump self) -> &'bump VNode<'bump> {
        todo!()
    }
}

pub struct BumpFrame {
    pub bump: Bump,
    pub head_node: VNode<'static>,
}

pub struct ActiveFrame {
    pub idx: AtomicUsize,
    pub frames: [BumpFrame; 2],
}

impl ActiveFrame {
    fn from_frames(a: BumpFrame, b: BumpFrame) -> Self {
        Self {
            idx: 0.into(),
            frames: [a, b],
        }
    }

    fn current_head_node<'b>(&'b self) -> &'b VNode<'b> {
        let cur_idx = self.idx.borrow().load(std::sync::atomic::Ordering::Relaxed);
        let raw_node = &self.frames[cur_idx];
        unsafe {
            let unsafe_head = &raw_node.head_node;
            let safe_node = std::mem::transmute::<&VNode<'static>, &VNode<'b>>(unsafe_head);
            safe_node
        }
    }

    fn next(&mut self) -> &mut BumpFrame {
        self.idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let cur = self.idx.borrow().load(std::sync::atomic::Ordering::Relaxed);
        match cur % 1 {
            1 => &mut self.frames[1],
            0 => &mut self.frames[0],
            _ => unreachable!("mod cannot by non-zero"),
        }
    }
}
