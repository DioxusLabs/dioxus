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

pub struct BumpContainer(pub UnsafeCell<Bump>);
impl BumpContainer {
    fn new() -> Self {
        Self(UnsafeCell::new(Bump::new()))
    }
}

impl Deref for BumpContainer {
    type Target = Bump;

    fn deref(&self) -> &Self::Target {
        todo!()
        // self.0.borrow()
    }
}
impl DerefMut for BumpContainer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        todo!()
        // self.0.borrow_mut()
    }
}
unsafe impl StableAddress for BumpContainer {}

#[ouroboros::self_referencing]
pub struct BumpFrame {
    pub bump: BumpContainer,

    #[covariant]
    #[borrows(bump)]
    pub head_node: &'this VNode<'this>,
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

    fn next(&self) -> &BumpFrame {
        self.idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let cur = self.idx.borrow().load(std::sync::atomic::Ordering::Relaxed);
        match cur % 1 {
            1 => &self.frames[1],
            0 => &self.frames[0],
            _ => unreachable!("mod cannot by non-zero"),
        }
    }
    // fn next(&self) -> &BumpFrame {
    //     self.idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    //     let cur = self.idx.borrow().load(std::sync::atomic::Ordering::Relaxed);
    //     match cur % 2_usize {
    //         1 => &self.frames[1],
    //         0 => &self.frames[0],
    //     }
    // }
}

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
    // somehow build this vnode with a lifetime tied to self
    // This root node has  "static" lifetime, but it's really not static.
    // It's goverened by the oldest of the two frames and is switched every time a new render occurs
    // Use this node as if it were static is unsafe, and needs to be fixed with ourborous or owning ref
    // ! do not copy this reference are things WILL break !
    pub frames: ActiveFrame,

    // IE Which listeners need to be woken up?
    pub listeners: Vec<Box<dyn Fn()>>,

    //
    pub props_type: TypeId,
    pub caller: *const i32,
}

// pub enum ActiveFrame {
//     First,
//     Second,
// }

// impl ActiveFrame {
//     fn next(&mut self) {
//         match self {
//             ActiveFrame::First => *self = ActiveFrame::Second,
//             ActiveFrame::Second => *self = ActiveFrame::First,
//         }
//     }
// }

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

        let listeners = Vec::new();

        let new_frame = BumpFrameBuilder {
            bump: BumpContainer::new(),
            head_node_builder: |bump| bump.alloc(VNode::text("")),
        }
        .build();

        let old_frame = BumpFrameBuilder {
            bump: BumpContainer::new(),
            head_node_builder: |bump| bump.alloc(VNode::text("")),
        }
        .build();

        let frames = ActiveFrame::from_frames(old_frame, new_frame);

        Self {
            hook_arena,
            hooks,
            props_type,
            caller,
            frames,
            listeners,
            parent,
        }
    }

    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    ///
    /// Props is ?Sized because we borrow the props and don't need to know the size. P (sized) is used as a marker (unsized)
    pub(crate) fn run<'a, 'bump, P: Properties + ?Sized>(&'bump mut self, props: &'a P) {
        // I really wanted to do this safely, but I don't think we can.
        // We want to reset the bump before writing into it. This requires &mut to the bump
        // Ouroborous lets us borrow with self, but the heads (IE the source) cannot be changed while the ref is live

        // n.b, there might be a better way of doing this active frame stuff - perhaps swapping
        let frame = self.frames.next();

        frame.with_bump(|bump_container| {
            let bump: &mut Bump = unsafe { &mut *bump_container.0.get() };
            bump.reset();

            let bump = &*bump;

            let ctx: Context<'bump> = Context {
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
            let nodes: &'bump VNode  = caller(ctx, props);
        });

        // let new_nodes = caller(ctx, props);
        // let r = new_nodes as *const _;
        // self.old_root = self.new_root;
        // self.new_root = new_nodes as *const _;

        // let old_nodes: &mut VNode<'static> = unsafe { &mut *self.root_node };

        // TODO: Iterate through the new nodes
        // move any listeners into ourself

        // perform the diff, dumping into the mutable change list
        // this doesnt perform any "diff compression" where an event and a re-render
        // crate::diff::diff(old_nodes, &new_nodes);
        todo!()
    }
}
