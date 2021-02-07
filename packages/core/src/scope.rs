use crate::nodes::VNode;
use crate::prelude::*;
use any::Any;
use bumpalo::Bump;
use generational_arena::{Arena, Index};
use std::{
    any::{self, TypeId},
    cell::{RefCell, UnsafeCell},
    future::Future,
    marker::PhantomData,
    sync::atomic::AtomicUsize,
};

/// The Scope that wraps a functional component
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components
/// The actualy contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
pub struct Scope {
    arena: typed_arena::Arena<Hook>,
    hooks: RefCell<Vec<*mut Hook>>,
    props_type: TypeId,
    caller: *const i32,
}

impl Scope {
    // create a new scope from a function
    pub fn new<T: 'static>(f: FC<T>) -> Self {
        // Capture the props type
        let props_type = TypeId::of::<T>();
        let arena = typed_arena::Arena::new();
        let hooks = RefCell::new(Vec::new());

        let caller = f as *const i32;

        Self {
            arena,
            hooks,
            props_type,
            caller,
        }
    }

    pub fn create_context<T: Properties>(&mut self) -> Context<T> {
        Context {
            _p: PhantomData {},
            arena: &self.arena,
            hooks: &self.hooks,
            idx: 0.into(),
            props: T::new(),
        }
    }

    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    fn run<T: 'static>(&self, f: FC<T>) {}

    fn call<T: Properties + 'static>(&mut self, val: T) {
        if self.props_type == TypeId::of::<T>() {
            /*
            SAFETY ALERT

            This particular usage of transmute is outlined in its docs https://doc.rust-lang.org/std/mem/fn.transmute.html
            We hide the generic bound on the function item by casting it to raw pointer. When the function is actually called,
            we transmute the function back using the props as reference.

            This is safe because we check that the generic type matches before casting.
            */
            let caller = unsafe { std::mem::transmute::<*const i32, FC<T>>(self.caller) };
            let ctx = self.create_context::<T>();
            // TODO: do something with these nodes
            let nodes = caller(ctx);
        } else {
            panic!("Do not try to use `call` on Scopes with the wrong props type")
        }
    }
}

/// Components in Dioxus use the "Context" object to interact with their lifecycle.
/// This lets components schedule updates, integrate hooks, and expose their context via the context api.
///
/// Properties passed down from the parent component are also directly accessible via the exposed "props" field.
///
/// ```ignore
/// #[derive(Properties)]
/// struct Props {
///     name: String
///
/// }
///
/// fn example(ctx: &Context<Props>) -> VNode {
///     html! {
///         <div> "Hello, {ctx.props.name}" </div>
///     }
/// }
/// ```
// todo: force lifetime of source into T as a valid lifetime too
// it's definitely possible, just needs some more messing around
pub struct Context<'src, T> {
    /// Direct access to the properties used to create this component.
    pub props: T,
    pub idx: AtomicUsize,

    // Borrowed from scope
    arena: &'src typed_arena::Arena<Hook>,
    hooks: &'src RefCell<Vec<*mut Hook>>,

    // holder for the src lifetime
    // todo @jon remove this
    pub _p: std::marker::PhantomData<&'src ()>,
}

impl<'a, T> Context<'a, T> {
    /// Access the children elements passed into the component
    pub fn children(&self) -> Vec<VNode> {
        todo!("Children API not yet implemented for component Context")
    }

    /// Access a parent context
    pub fn parent_context<C>(&self) -> C {
        todo!("Context API is not ready yet")
    }

    /// Create a subscription that schedules a future render for the reference component
    pub fn subscribe(&self) -> impl FnOnce() -> () {
        todo!("Subscription API is not ready yet");
        || {}
    }

    /// Take a lazy VNode structure and actually build it with the context of the VDom's efficient VNode allocator.
    ///
    /// ```ignore
    /// fn Component(ctx: Context<Props>) -> VNode {
    ///     // Lazy assemble the VNode tree
    ///     let lazy_tree = html! {<div>"Hello World"</div>};
    ///     
    ///     // Actually build the tree and allocate it
    ///     ctx.view(lazy_tree)
    /// }
    ///```
    pub fn view(&self, v: impl FnOnce(&'a Bump) -> VNode<'a>) -> VNode<'a> {
        todo!()
    }

    /// Create a suspended component from a future.
    ///
    /// When the future completes, the component will be renderered
    pub fn suspend(
        &self,
        fut: impl Future<Output = impl FnOnce(&'a Bump) -> VNode<'a>>,
    ) -> VNode<'a> {
        todo!()
    }

    /// use_hook provides a way to store data between renders for functional components.
    pub fn use_hook<'comp, InternalHookState: 'static, Output: 'comp>(
        &'comp self,
        // The closure that builds the hook state
        initializer: impl FnOnce() -> InternalHookState,
        // The closure that takes the hookstate and returns some value
        runner: impl FnOnce(&'comp mut InternalHookState, ()) -> Output,
        // The closure that cleans up whatever mess is left when the component gets torn down
        // TODO: add this to the "clean up" group for when the component is dropped
        cleanup: impl FnOnce(InternalHookState),
    ) -> Output {
        let raw_hook = {
            let idx = self.idx.load(std::sync::atomic::Ordering::Relaxed);

            // Mutate hook list if necessary
            let mut hooks = self.hooks.borrow_mut();

            // Initialize the hook by allocating it in the typed arena.
            // We get a reference from the arena which is owned by the component scope
            // This is valid because "Context" is only valid while the scope is borrowed
            if idx >= hooks.len() {
                let new_state = initializer();
                let boxed_state: Box<dyn std::any::Any> = Box::new(new_state);
                let hook = self.arena.alloc(Hook::new(boxed_state));

                // Push the raw pointer instead of the &mut
                // A "poor man's OwningRef"
                hooks.push(hook);
            }
            self.idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            *hooks.get(idx).unwrap()
        };

        /*
        ** UNSAFETY ALERT **
        Here, we dereference a raw pointer. Normally, we aren't guaranteed that this is okay.

        However, typed-arena gives a mutable reference to the stored data which is stable for any inserts
        into the arena. During the first call of the function, we need to add the mutable reference given to us by
        the arena into our list of hooks. The arena provides stability of the &mut references and is only deallocated
        when the component itself is deallocated.

        This is okay because:
        - The lifetime of the component arena is tied to the lifetime of these raw hooks
        - Usage of the raw hooks is tied behind the Vec refcell
        - Output is static, meaning it can't take a reference to the data
        - We don't expose the raw hook pointer outside of the scope of use_hook
        - The reference is tied to context, meaning it can only be used while ctx is around to free it
        */
        let borrowed_hook: &'comp mut _ = unsafe { raw_hook.as_mut().unwrap() };

        let internal_state = borrowed_hook
            .state
            .downcast_mut::<InternalHookState>()
            .unwrap();

        // todo: set up an updater with the subscription API
        let updater = ();

        runner(internal_state, updater)
    }
}

pub struct Hook {
    state: Box<dyn std::any::Any>,
}

impl Hook {
    fn new(state: Box<dyn std::any::Any>) -> Self {
        Self { state }
    }
}
