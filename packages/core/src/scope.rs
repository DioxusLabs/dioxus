use crate::context::hooks::Hook;
use crate::innerlude::*;
use crate::nodes::VNode;
use bumpalo::Bump;
use generational_arena::Index;

use std::{
    any::TypeId,
    borrow::{Borrow, BorrowMut},
    cell::{RefCell, UnsafeCell},
    future::Future,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::AtomicUsize,
    sync::atomic::Ordering,
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

    pub frames: ActiveFrame,

    // IE Which listeners need to be woken up?
    pub listeners: Vec<Box<dyn Fn(crate::events::VirtualEvent)>>,

    // lying, cheating reference >:(
    pub props: Box<dyn std::any::Any>,

    // pub props_type: TypeId,
    pub caller: *const (),
}

impl Scope {
    // create a new scope from a function
    pub fn new<'a, P1, P2: 'static>(f: FC<P1>, props: P1, parent: Option<Index>) -> Self {
        let hook_arena = typed_arena::Arena::new();
        let hooks = RefCell::new(Vec::new());

        // Capture the caller
        let caller = f as *const ();

        let listeners: Vec<Box<dyn Fn(crate::events::VirtualEvent)>> = vec![Box::new(|_| {
            log::info!("Base listener called");
        })];

        let old_frame = BumpFrame {
            bump: Bump::new(),
            head_node: VNode::text(""),
        };

        let new_frame = BumpFrame {
            bump: Bump::new(),
            head_node: VNode::text(""),
        };

        let frames = ActiveFrame::from_frames(old_frame, new_frame);

        // box the props
        let props = Box::new(props);

        let props = unsafe { std::mem::transmute::<_, Box<P2>>(props) };

        Self {
            hook_arena,
            hooks,
            caller,
            frames,
            listeners,
            parent,
            props,
        }
    }

    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    ///
    /// Props is ?Sized because we borrow the props and don't need to know the size. P (sized) is used as a marker (unsized)
    pub fn run<'bump, PLocked: Sized + 'static>(&'bump mut self) {
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
        };

        unsafe {
            /*
            SAFETY ALERT

            This particular usage of transmute is outlined in its docs https://doc.rust-lang.org/std/mem/fn.transmute.html
            We hide the generic bound on the function item by casting it to raw pointer. When the function is actually called,
            we transmute the function back using the props as reference.

            we could do a better check to make sure that the TypeID is correct before casting
            --
            This is safe because we check that the generic type matches before casting.
            */
            // we use plocked to be able to remove the borrowed lifetime
            // these lifetimes could be very broken, so we need to dynamically manage them
            let caller = std::mem::transmute::<*const (), FC<PLocked>>(self.caller);
            let props = self.props.downcast_ref::<PLocked>().unwrap();

            // Note that the actual modification of the vnode head element occurs during this call
            let _nodes: DomTree = caller(ctx, props);

            /*
            SAFETY ALERT

            DO NOT USE THIS VNODE WITHOUT THE APPOPRIATE ACCESSORS.
            KEEPING THIS STATIC REFERENCE CAN LEAD TO UB.

            Some things to note:
            - The VNode itself is bound to the lifetime, but it itself is owned by scope.
            - The VNode has a private API and can only be used from accessors.
            - Public API cannot drop or destructure VNode
            */
            // the nodes we care about have been unsafely extended to a static lifetime in context
            frame.head_node = node_slot
                .deref()
                .borrow_mut()
                .take()
                .expect("Viewing did not happen");
        }
    }

    /// Accessor to get the root node and its children (safely)\
    /// Scope is self-referntial, so we are forced to use the 'static lifetime to cheat
    pub fn new_frame<'bump>(&'bump self) -> &'bump VNode<'bump> {
        self.frames.current_head_node()
    }

    pub fn old_frame<'bump>(&'bump self) -> &'bump VNode<'bump> {
        self.frames.prev_head_node()
    }
}

// todo, do better with the active frame stuff
// somehow build this vnode with a lifetime tied to self
// This root node has  "static" lifetime, but it's really not static.
// It's goverened by the oldest of the two frames and is switched every time a new render occurs
// Use this node as if it were static is unsafe, and needs to be fixed with ourborous or owning ref
// ! do not copy this reference are things WILL break !
pub struct ActiveFrame {
    pub idx: AtomicUsize,
    pub frames: [BumpFrame; 2],
}

pub struct BumpFrame {
    pub bump: Bump,
    pub head_node: VNode<'static>,
}

impl ActiveFrame {
    fn from_frames(a: BumpFrame, b: BumpFrame) -> Self {
        Self {
            idx: 0.into(),
            frames: [a, b],
        }
    }

    fn current_head_node<'b>(&'b self) -> &'b VNode<'b> {
        let raw_node = match self.idx.borrow().load(Ordering::Relaxed) & 1 != 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        };
        unsafe {
            let unsafe_head = &raw_node.head_node;
            let safe_node = std::mem::transmute::<&VNode<'static>, &VNode<'b>>(unsafe_head);
            safe_node
        }
    }

    fn prev_head_node<'b>(&'b self) -> &'b VNode<'b> {
        let raw_node = match self.idx.borrow().load(Ordering::Relaxed) & 1 == 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        };

        unsafe {
            let unsafe_head = &raw_node.head_node;
            let safe_node = std::mem::transmute::<&VNode<'static>, &VNode<'b>>(unsafe_head);
            safe_node
        }
    }

    fn next(&mut self) -> &mut BumpFrame {
        self.idx.fetch_add(1, Ordering::Relaxed);
        let cur = self.idx.borrow().load(Ordering::Relaxed);
        match cur % 1 {
            1 => &mut self.frames[1],
            0 => &mut self.frames[0],
            _ => unreachable!("mod cannot by non-zero"),
        }
    }
}

// #[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::bumpalo;
    // use crate::prelude::bumpalo::collections::string::String;
    use crate::prelude::format_args_f;

    static ListenerTest: FC<()> = |ctx, props| {
        ctx.view(html! {
            <div onclick={|_| println!("Hell owlrld")}>
                "hello"
            </div>
        })
    };

    #[test]
    fn check_listeners() -> Result<()> {
        let mut scope = Scope::new::<(), ()>(ListenerTest, (), None);
        scope.run::<()>();

        let nodes = scope.new_frame();
        dbg!(nodes);

        Ok(())
    }

    #[test]
    fn test_scope() {
        let example: FC<()> = |ctx, props| {
            use crate::builder::*;
            ctx.view(|b| div(b).child(text("a")).finish())
        };

        let props = ();
        let parent = None;
        let scope = Scope::new::<(), ()>(example, props, parent);
    }

    #[derive(Debug)]
    struct ExampleProps<'src> {
        name: &'src String,
    }
    // impl<'src> Properties<'src> for ExampleProps<'src> {}

    #[derive(Debug)]
    struct EmptyProps<'src> {
        name: &'src String,
    }
    // impl<'src> Properties<'src> for EmptyProps<'src> {}

    use crate::{builder::*, hooks::use_ref};

    fn example_fc<'a>(ctx: Context<'a>, props: &'a EmptyProps) -> DomTree {
        // fn example_fc<'a>(ctx: Context<'a>, props: &'a EmptyProps<'a>) -> DomTree {
        let (content, _): (&'a String, _) = crate::hooks::use_state(&ctx, || "abcd".to_string());
        // let (content, _): (&'a String, _) = crate::hooks::use_state(&ctx, || "abcd".to_string());
        // let (text, set_val) = crate::hooks::use_state(&ctx, || "abcd".to_string());

        let childprops: ExampleProps<'a> = ExampleProps { name: content };
        // let childprops: ExampleProps<'a> = ExampleProps { name: content };
        ctx.view(move |b: &'a Bump| {
            div(b)
                .child(text(props.name))
                // .child(text(props.name))
                .child(virtual_child::<ExampleProps>(b, childprops, child_example))
                // .child(virtual_child::<ExampleProps<'a>>(b, childprops, CHILD))
                // as for<'scope> fn(Context<'_>, &'scope ExampleProps<'scope>) -> DomTree
                // |ctx, pops| todo!(),
                // .child(virtual_child::<'a>(
                //     b,
                //     child_example,
                //     ExampleProps { name: text },
                // ))
                .finish()
        })
    }

    fn child_example<'b>(ctx: Context<'b>, props: &'b ExampleProps) -> DomTree {
        ctx.view(move |b| {
            div(b)
                .child(text(props.name))
                //
                .finish()
        })
    }

    static CHILD: FC<ExampleProps> = |ctx, props: &'_ ExampleProps| {
        // todo!()
        ctx.view(move |b| {
            div(b)
                .child(text(props.name))
                //
                .finish()
        })
    };
    #[test]
    fn test_borrowed_scope() {
        // use crate::builder::*;

        let example: FC<EmptyProps> = |ctx, props| {
            // render counter
            // let mut val = crate::hooks::use_ref(&ctx, || 0);
            // val.modify(|f| {
            //     *f += 1;
            // });
            // dbg!(val.current());
            // only needs to be valid when ran?
            // can only borrow from parent?
            // props are boxed in parent's scope?
            // passing complex structures down to child?
            // stored value
            // let (text, set_val) = crate::hooks::use_state(&ctx, || "abcd".to_string());

            ctx.view(move |b| {
                todo!()
                // div(b)
                //     // .child(text(props.name))
                //     // .child(virtual_child(b, CHILD, ExampleProps { name: val.as_str() }))
                //     .child(virtual_child(b, CHILD, ExampleProps { name:  }))
                //     .finish()
            })
        };

        let source_text = "abcd123".to_string();
        let props = ExampleProps { name: &source_text };

        // let parent = None;
        // let mut scope =
        //     Scope::new::<EmptyProps, EmptyProps>(example, EmptyProps { name: &() }, parent);
        // scope.run::<ExampleProps>();
        // scope.run::<ExampleProps>();
        // scope.run::<ExampleProps>();
        // scope.run::<ExampleProps>();
        // scope.run::<ExampleProps>();
        // let nodes = scope.current_root_node();
        // dbg!(nodes);
    }
}
