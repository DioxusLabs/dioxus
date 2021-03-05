use crate::component::ScopeIdx;
use crate::context::hooks::Hook;
use crate::innerlude::*;
use crate::nodes::VNode;
use bumpalo::Bump;

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    marker::PhantomData,
    ops::Deref,
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

    // lying, cheating reference >:(
    pub props: Box<dyn std::any::Any>,

    // our own index
    pub myidx: ScopeIdx,

    // ==========================
    // slightly unsafe stuff
    // ==========================
    // an internal, highly efficient storage of vnodes
    pub frames: ActiveFrame,

    // These hooks are actually references into the hook arena
    // These two could be combined with "OwningRef" to remove unsafe usage
    // could also use ourborous
    pub hooks: RefCell<Vec<*mut Hook>>,
    pub hook_arena: typed_arena::Arena<Hook>,

    // Unsafety:
    // - is self-refenrential and therefore needs to point into the bump
    //
    // Stores references into the listeners attached to the vnodes
    // NEEDS TO BE PRIVATE
    listeners: RefCell<Vec<*const dyn Fn(crate::events::VirtualEvent)>>,

    // Unsafety
    // - is a raw ptr because we need to compare
    pub caller: *const (),
}

impl Scope {
    // create a new scope from a function
    pub fn new<'a, P1, P2: 'static>(
        f: FC<P1>,
        props: P1,
        myidx: ScopeIdx,
        parent: Option<ScopeIdx>,
    ) -> Self {
        let hook_arena = typed_arena::Arena::new();
        let hooks = RefCell::new(Vec::new());

        // Capture the caller
        let caller = f as *const ();

        let listeners = Default::default();

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
            myidx,
            hook_arena,
            hooks,
            caller,
            frames,
            listeners,
            parent,
            props,
        }
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
            log::debug!("Running listener");

            // drain all the event listeners
            // if we don't, then they'll stick around and become invalid
            // big big big big safety issue
            self.listeners.borrow_mut().drain(..);
        }
    }
}

pub struct RawComponent {
    // used as a memoization strategy
    comparator: *const Box<dyn Fn(&Box<dyn Any>) -> bool>,

    // used to actually run the component
    // encapsulates props
    runner: *const Box<dyn Fn(Context) -> DomTree>,

    // the actual FC<T>
    raw: *const (),
}

impl Scope {
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
            scope: self.myidx,
            listeners: &self.listeners,
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
            let _: DomTree = caller(ctx, props);

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
}

fn retrieve_listeners(node: &VNode<'static>, listeners: &mut Vec<&Listener>) {
    if let VNode::Element(el) = *node {
        for listener in el.listeners {
            // let g = listener as *const Listener;
            listeners.push(listener);
        }
        for child in el.children {
            retrieve_listeners(child, listeners);
        }
    }
}

// ==========================
// Active-frame related code
// ==========================
impl Scope {
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
    pub idx: RefCell<usize>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    static ListenerTest: FC<()> = |ctx, props| {
        ctx.render(html! {
            <div onclick={|_| println!("Hell owlrld")}>
                "hello"
            </div>
        })
    };

    #[test]
    fn test_scope() {
        let example: FC<()> = |ctx, props| {
            use crate::builder::*;
            ctx.render(|ctx| {
                builder::ElementBuilder::new(ctx, "div")
                    .child(text("a"))
                    .finish()
            })
        };

        let props = ();
        let parent = None;
        let mut nodes = generational_arena::Arena::new();
        nodes.insert_with(|f| {
            let scope = Scope::new::<(), ()>(example, props, f, parent);
        });
    }

    #[derive(Debug)]
    struct ExampleProps<'src> {
        name: &'src String,
    }

    #[derive(Debug)]
    struct EmptyProps<'src> {
        name: &'src String,
    }

    use crate::{builder::*, hooks::use_ref};

    fn example_fc<'a>(ctx: Context<'a>, props: &'a EmptyProps) -> DomTree {
        let (content, _): (&'a String, _) = crate::hooks::use_state(&ctx, || "abcd".to_string());

        let childprops: ExampleProps<'a> = ExampleProps { name: content };
        ctx.render(move |c| {
            builder::ElementBuilder::new(c, "div")
                .child(text(props.name))
                .child(virtual_child::<ExampleProps>(
                    c.bump,
                    childprops,
                    child_example,
                ))
                .finish()
        })
    }

    fn child_example<'b>(ctx: Context<'b>, props: &'b ExampleProps) -> DomTree {
        ctx.render(move |ctx| {
            builder::ElementBuilder::new(ctx, "div")
                .child(text(props.name))
                .finish()
        })
    }

    static CHILD: FC<ExampleProps> = |ctx, props: &'_ ExampleProps| {
        ctx.render(move |ctx| {
            builder::ElementBuilder::new(ctx, "div")
                .child(text(props.name))
                .finish()
        })
    };

    #[test]
    fn test_borrowed_scope() {
        let example: FC<EmptyProps> = |ctx, props| {
            ctx.render(move |b| {
                builder::ElementBuilder::new(b, "div")
                    .child(virtual_child(
                        b.bump,
                        ExampleProps { name: props.name },
                        CHILD,
                    ))
                    .finish()
            })
        };

        let source_text = "abcd123".to_string();
        let props = ExampleProps { name: &source_text };
    }
}
