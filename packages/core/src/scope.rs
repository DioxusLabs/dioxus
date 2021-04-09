use crate::{innerlude::*, virtual_dom::OpaqueComponent};
use bumpalo::Bump;

use std::{
    cell::RefCell,
    collections::HashSet,
    marker::PhantomData,
    rc::{Rc, Weak},
};

/// Every component in Dioxus is represented by a `Scope`.
///
/// Scopes contain the state for hooks, the component's props, and other lifecycle information.
///
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components.
/// The actual contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
pub struct Scope {
    // Map to the parent
    pub parent: Option<ScopeIdx>,

    // our own index
    pub myidx: ScopeIdx,

    //
    pub children: HashSet<ScopeIdx>,

    pub caller: Weak<OpaqueComponent<'static>>,

    // ==========================
    // slightly unsafe stuff
    // ==========================
    // an internal, highly efficient storage of vnodes
    pub frames: ActiveFrame,

    // These hooks are actually references into the hook arena
    // These two could be combined with "OwningRef" to remove unsafe usage
    // or we could dedicate a tiny bump arena just for them
    // could also use ourborous
    pub hooks: RefCell<Vec<Hook>>,

    pub hook_arena: Vec<Hook>,

    // Unsafety:
    // - is self-refenrential and therefore needs to point into the bump
    // Stores references into the listeners attached to the vnodes
    // NEEDS TO BE PRIVATE
    pub(crate) listeners: RefCell<Vec<*const dyn Fn(VirtualEvent)>>,
}

impl Scope {
    // we are being created in the scope of an existing component (where the creator_node lifetime comes into play)
    // we are going to break this lifetime by force in order to save it on ourselves.
    // To make sure that the lifetime isn't truly broken, we receive a Weak RC so we can't keep it around after the parent dies.
    // This should never happen, but is a good check to keep around
    pub fn new<'creator_node>(
        caller: Weak<OpaqueComponent<'creator_node>>,
        myidx: ScopeIdx,
        parent: Option<ScopeIdx>,
    ) -> Self {
        // Caller has been broken free
        // However, it's still weak, so if the original Rc gets killed, we can't touch it
        let broken_caller: Weak<OpaqueComponent<'static>> = unsafe { std::mem::transmute(caller) };

        Self {
            caller: broken_caller,
            hook_arena: Vec::new(),
            hooks: RefCell::new(Vec::new()),
            frames: ActiveFrame::new(),
            children: HashSet::new(),
            listeners: Default::default(),
            parent,
            myidx,
        }
    }

    pub fn update_caller<'creator_node>(&mut self, caller: Weak<OpaqueComponent<'creator_node>>) {
        let broken_caller: Weak<OpaqueComponent<'static>> = unsafe { std::mem::transmute(caller) };

        self.caller = broken_caller;
    }

    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    ///
    /// Props is ?Sized because we borrow the props and don't need to know the size. P (sized) is used as a marker (unsized)
    pub fn run_scope<'b>(&'b mut self) -> Result<()> {
        // cycle to the next frame and then reset it
        // this breaks any latent references
        self.frames.next().bump.reset();

        let ctx = Context {
            idx: 0.into(),
            _p: PhantomData {},
            scope: self,
        };

        let caller = self.caller.upgrade().expect("Failed to get caller");

        /*
        SAFETY ALERT

        DO NOT USE THIS VNODE WITHOUT THE APPOPRIATE ACCESSORS.
        KEEPING THIS STATIC REFERENCE CAN LEAD TO UB.

        Some things to note:
        - The VNode itself is bound to the lifetime, but it itself is owned by scope.
        - The VNode has a private API and can only be used from accessors.
        - Public API cannot drop or destructure VNode
        */
        let new_head = unsafe {
            // use the same type, just manipulate the lifetime
            type ComComp<'c> = Rc<OpaqueComponent<'c>>;
            let caller = std::mem::transmute::<ComComp<'static>, ComComp<'b>>(caller);
            (caller.as_ref())(ctx)
        };

        self.frames.cur_frame_mut().head_node = new_head.root;
        Ok(())
    }

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
            log::debug!("Listener finished");

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

    pub fn cur_frame(&self) -> &BumpFrame {
        self.frames.cur_frame()
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

    fn cur_frame(&self) -> &BumpFrame {
        match *self.idx.borrow() & 1 == 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }
    fn cur_frame_mut(&mut self) -> &mut BumpFrame {
        match *self.idx.borrow() & 1 == 0 {
            true => &mut self.frames[0],
            false => &mut self.frames[1],
        }
    }

    pub fn current_head_node<'b>(&'b self) -> &'b VNode<'b> {
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

    pub fn prev_head_node<'b>(&'b self) -> &'b VNode<'b> {
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
