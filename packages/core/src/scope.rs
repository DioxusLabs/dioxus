use crate::innerlude::*;
use bumpalo::Bump;

use std::{
    any::Any,
    cell::RefCell,
    collections::HashSet,
    marker::PhantomData,
    ops::Deref,
    rc::{Rc, Weak},
};

// pub trait Scoped {
//     fn run(&mut self);
//     // fn compare_props(&self, new: &dyn std::any::Any) -> bool;
//     fn call_listener(&mut self, trigger: EventTrigger);
//     fn new_frame<'bump>(&'bump self) -> &'bump VNode<'bump>;
//     fn old_frame<'bump>(&'bump self) -> &'bump VNode<'bump>;
// }

/// Every component in Dioxus is represented by a `Scope`.
///
/// Scopes contain the state for hooks, the component's props, and other lifecycle information.
///
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components.
/// The actual contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
pub struct Scope {
    // pub struct Scope<P: Properties> {
    // Map to the parent
    pub parent: Option<ScopeIdx>,

    // our own index
    pub myidx: ScopeIdx,

    //
    pub children: HashSet<ScopeIdx>,

    pub caller: Weak<dyn Fn(Context) -> DomTree + 'static>,

    // // the props
    // pub props: P,

    // and the actual render function
    // pub caller: *const dyn Fn(Context) -> DomTree,
    // _p: std::marker::PhantomData<P>,
    // _p: std::marker::PhantomData<P>,
    // pub raw_caller: FC<P>,

    // ==========================
    // slightly unsafe stuff
    // ==========================
    // an internal, highly efficient storage of vnodes
    pub frames: ActiveFrame,

    // These hooks are actually references into the hook arena
    // These two could be combined with "OwningRef" to remove unsafe usage
    // or we could dedicate a tiny bump arena just for them
    // could also use ourborous
    pub hooks: RefCell<Vec<*mut Hook>>,
    pub hook_arena: typed_arena::Arena<Hook>,

    // Unsafety:
    // - is self-refenrential and therefore needs to point into the bump
    // Stores references into the listeners attached to the vnodes
    // NEEDS TO BE PRIVATE
    listeners: RefCell<Vec<*const dyn Fn(VirtualEvent)>>,
}

// instead of having it as a trait method, we use a single function
// todo: do the unsafety magic stuff to erase the type of p
// pub fn create_scoped(
//     // raw_f: FC<P>,
//     // caller: Caller,

//     // props: P,
//     myidx: ScopeIdx,
//     parent: Option<ScopeIdx>,
// ) -> Box<dyn Scoped> {
//     Box::new(Scope::<()> {
//         // raw_caller: raw_f,
//         _p: Default::default(),
//         // caller,
//         myidx,
//         hook_arena: typed_arena::Arena::new(),
//         hooks: RefCell::new(Vec::new()),
//         frames: ActiveFrame::new(),
//         children: HashSet::new(),
//         listeners: Default::default(),
//         parent,
//         // props,
//     })
// }

impl Scope {
    // we are being created in the scope of an existing component (where the creator_node lifetime comes into play)
    // we are going to break this lifetime by force in order to save it on ourselves.
    // To make sure that the lifetime isn't truly broken, we receive a Weak RC so we can't keep it around after the parent dies.
    // This should never happen, but is a good check to keep around
    pub fn new<'creator_node>(
        // pub fn new(
        // caller: Weak<dyn Fn(Context) -> DomTree + 'static>,
        caller: Weak<dyn Fn(Context) -> DomTree + 'creator_node>,
        myidx: ScopeIdx,
        parent: Option<ScopeIdx>,
    ) -> Self {
        // caller has been broken free
        // however, it's still weak, so if the original Rc gets killed, we can't touch it
        let broken_caller: Weak<dyn Fn(Context) -> DomTree + 'static> =
            unsafe { std::mem::transmute(caller) };

        // let broken_caller = caller;
        Self {
            caller: broken_caller,
            hook_arena: typed_arena::Arena::new(),
            hooks: RefCell::new(Vec::new()),
            frames: ActiveFrame::new(),
            children: HashSet::new(),
            listeners: Default::default(),
            parent,
            myidx,
        }
    }

    // impl<P: Properties + 'static> Scoped for Scope<P> {
    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    ///
    /// Props is ?Sized because we borrow the props and don't need to know the size. P (sized) is used as a marker (unsized)
    pub fn run_scope<'bump>(&'bump mut self) -> Result<()> {
        let frame = {
            let frame = self.frames.next();
            frame.bump.reset();
            frame
        };

        let node_slot = std::rc::Rc::new(RefCell::new(None));

        let ctx: Context<'bump> = Context {
            arena: &self.hook_arena,
            hooks: &self.hooks,
            bump: &frame.bump,
            idx: 0.into(),
            _p: PhantomData {},
            final_nodes: node_slot.clone(),
            scope: self.myidx,
            listeners: &self.listeners,
        };

        todo!()
        // Note that the actual modification of the vnode head element occurs during this call
        // let _: DomTree = (self.caller.0.as_ref())(ctx);
        // let _: DomTree = (self.raw_caller)(ctx, &self.props);

        /*
        SAFETY ALERT

        DO NOT USE THIS VNODE WITHOUT THE APPOPRIATE ACCESSORS.
        KEEPING THIS STATIC REFERENCE CAN LEAD TO UB.

        Some things to note:
        - The VNode itself is bound to the lifetime, but it itself is owned by scope.
        - The VNode has a private API and can only be used from accessors.
        - Public API cannot drop or destructure VNode
        */

        // frame.head_node = node_slot
        //     .deref()
        //     .borrow_mut()
        //     .take()
        //     .expect("Viewing did not happen");
    }

    // pub fn compare_props(&self, new: &dyn Any) -> bool {
    //     new.downcast_ref::<P>()
    //         .map(|f| &self.props == f)
    //         .expect("Props should not be of a different type")
    // }

    // A safe wrapper around calling listeners
    // calling listeners will invalidate the list of listeners
    // The listener list will be completely drained because the next frame will write over previous listeners
    pub fn call_listener(&mut self, trigger: EventTrigger) {
        let EventTrigger {
            listener_id,
            event: source,
            ..
        } = trigger;

        unsafe {
            let listener = self
                .listeners
                .borrow()
                .get(listener_id as usize)
                .expect("Listener should exist if it was triggered")
                .as_ref()
                .unwrap();

            // Run the callback with the user event
            log::debug!("Running listener");
            listener(source);
            log::debug!("Running listener");

            // drain all the event listeners
            // if we don't, then they'll stick around and become invalid
            // big big big big safety issue
            self.listeners.borrow_mut().drain(..);
        }
    }

    pub fn new_frame<'bump>(&'bump self) -> &'bump VNode<'bump> {
        self.frames.current_head_node()
    }

    pub fn old_frame<'bump>(&'bump self) -> &'bump VNode<'bump> {
        self.frames.prev_head_node()
    }
}

// ==========================
// Active-frame related code
// ==========================
// todo, do better with the active frame stuff
// somehow build this vnode with a lifetime tied to self
// This root node has  "static" lifetime, but it's really not static.
// It's goverened by the oldest of the two frames and is switched every time a new render occurs
// Use this node as if it were static is unsafe, and needs to be fixed with ourborous or owning ref
// ! do not copy this reference are things WILL break !
pub struct ActiveFrame {
    pub idx: RefCell<usize>,
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

    fn from_frames(a: BumpFrame, b: BumpFrame) -> Self {
        Self {
            idx: 0.into(),
            frames: [a, b],
        }
    }

    fn current_head_node<'b>(&'b self) -> &'b VNode<'b> {
        let raw_node = match *self.idx.borrow() & 1 == 0 {
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

    fn prev_head_node<'b>(&'b self) -> &'b VNode<'b> {
        let raw_node = match *self.idx.borrow() & 1 != 0 {
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

    fn next(&mut self) -> &mut BumpFrame {
        *self.idx.borrow_mut() += 1;

        if *self.idx.borrow() % 2 == 0 {
            &mut self.frames[0]
        } else {
            &mut self.frames[1]
        }
    }
}

#[cfg(old)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    // static ListenerTest: FC<()> = |ctx, props| {
    //     ctx.render(html! {
    //         <div onclick={|_| println!("Hell owlrld")}>
    //             "hello"
    //         </div>
    //     })
    // };

    #[test]
    fn test_scope() {
        #[derive(PartialEq)]
        struct Example {}
        impl FC for Example {
            fn render(ctx: Context<'_>, _: &Self) -> DomTree {
                use crate::builder::*;
                ctx.render(|ctx| {
                    builder::ElementBuilder::new(ctx, "div")
                        .child(text("a"))
                        .finish()
                })
            }
        }

        let mut nodes = generational_arena::Arena::new();
        nodes.insert_with(|myidx| {
            let scope = create_scoped(Example {}, myidx, None);
        });
    }

    use crate::{builder::*, hooks::use_ref};

    #[derive(Debug, PartialEq)]
    struct EmptyProps<'src> {
        name: &'src String,
    }

    impl FC for EmptyProps<'_> {
        fn render(ctx: Context, props: &Self) -> DomTree {
            let (content, _): (&String, _) = crate::hooks::use_state(&ctx, || "abcd".to_string());

            let childprops: ExampleProps<'_> = ExampleProps { name: content };
            todo!()
            // ctx.render(move |c| {
            //     builder::ElementBuilder::new(c, "div")
            //         .child(text(props.name))
            //         .child(virtual_child(c, childprops))
            //         .finish()
            // })
        }
    }

    #[derive(Debug, PartialEq)]
    struct ExampleProps<'src> {
        name: &'src String,
    }

    impl FC for ExampleProps<'_> {
        fn render(ctx: Context, props: &Self) -> DomTree {
            ctx.render(move |ctx| {
                builder::ElementBuilder::new(ctx, "div")
                    .child(text(props.name))
                    .finish()
            })
        }
    }

    #[test]
    fn test_borrowed_scope() {
        #[derive(Debug, PartialEq)]
        struct Example {
            name: String,
        }

        impl FC for Example {
            fn render(ctx: Context, props: &Self) -> DomTree {
                todo!()
                // ctx.render(move |c| {
                //     builder::ElementBuilder::new(c, "div")
                //         .child(virtual_child(c, ExampleProps { name: &props.name }))
                //         .finish()
                // })
            }
        }

        let source_text = "abcd123".to_string();
        let props = ExampleProps { name: &source_text };
    }
}

#[cfg(asd)]
mod old {

    /// The ComponentCaller struct is an opaque object that encapsultes the memoization and running functionality for FC
    ///
    /// It's opaque because during the diffing mechanism, the type of props is sealed away in a closure. This makes it so
    /// scope doesn't need to be generic
    pub struct ComponentCaller {
        // used as a memoization strategy
        comparator: Box<dyn Fn(&Box<dyn Any>) -> bool>,

        // used to actually run the component
        // encapsulates props
        runner: Box<dyn Fn(Context) -> DomTree>,

        props_type: TypeId,

        // the actual FC<T>
        raw: *const (),
    }

    impl ComponentCaller {
        fn new<P>(props: P) -> Self {
            let comparator = Box::new(|f| false);
            todo!();
            // Self { comparator }
        }

        fn update_props<P>(props: P) {}
    }
}
