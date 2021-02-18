use crate::prelude::*;
use crate::{innerlude::Scope, nodes::VNode};
use bumpalo::Bump;
use hooks::Hook;
use std::{
    any::TypeId, borrow::Borrow, cell::RefCell, future::Future, marker::PhantomData,
    sync::atomic::AtomicUsize,
};

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
pub struct Context<'src> {
    pub idx: AtomicUsize,

    // Borrowed from scope
    pub(crate) arena: &'src typed_arena::Arena<Hook>,
    pub(crate) hooks: &'src RefCell<Vec<*mut Hook>>,
    pub(crate) bump: &'src Bump,

    // holder for the src lifetime
    // todo @jon remove this
    pub _p: std::marker::PhantomData<&'src ()>,
}

impl<'a> Context<'a> {
    // impl<'a, PropType> Context<'a, PropType> {
    /// Access the children elements passed into the component
    pub fn children(&self) -> Vec<VNode> {
        todo!("Children API not yet implemented for component Context")
    }

    /// Create a subscription that schedules a future render for the reference component
    pub fn schedule_update(&self) -> impl Fn() -> () {
        todo!("Subscription API is not ready yet");
        || {}
    }

    /// Take a lazy VNode structure and actually build it with the context of the VDom's efficient VNode allocator.
    ///
    /// This function consumes the context and absorb the lifetime, so these VNodes *must* be returned.
    ///
    /// ## Example
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
    pub fn view(self, lazy_nodes: impl FnOnce(&'a Bump) -> VNode<'a> + 'a) -> VNode<'a> {
        lazy_nodes(self.bump)
    }

    pub fn callback(&self, f: impl Fn(()) + 'static) {}

    /// Create a suspended component from a future.
    ///
    /// When the future completes, the component will be renderered
    pub fn suspend(
        &self,
        fut: impl Future<Output = impl FnOnce(&'a Bump) -> VNode<'a>>,
    ) -> VNode<'a> {
        todo!()
    }
}

pub mod hooks {
    //! This module provides internal state management functionality for Dioxus components
    //!

    use super::*;

    pub struct Hook(pub Box<dyn std::any::Any>);

    impl Hook {
        pub fn new(state: Box<dyn std::any::Any>) -> Self {
            Self(state)
        }
    }

    impl<'a> Context<'a> {
        /// TODO: @jon, rework this so we dont have to use unsafe to make hooks and then return them
        /// use_hook provides a way to store data between renders for functional components.
        /// todo @jon: ensure the hook arena is stable with pin or is stable by default
        pub fn use_hook<'internal, 'scope, InternalHookState: 'static, Output: 'internal>(
            &'scope self,
            // The closure that builds the hook state
            initializer: impl FnOnce() -> InternalHookState,
            // The closure that takes the hookstate and returns some value
            runner: impl FnOnce(&'internal mut InternalHookState) -> Output,
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
            - Output is static, meaning it  can't take a reference to the data
            - We don't expose the raw hook pointer outside of the scope of use_hook
            - The reference is tied to context, meaning it can only be used while ctx is around to free it
            */
            let borrowed_hook: &'internal mut _ = unsafe { raw_hook.as_mut().unwrap() };

            let internal_state = borrowed_hook.0.downcast_mut::<InternalHookState>().unwrap();

            runner(internal_state)
        }
    }
}

mod context_api {
    //! Context API
    //!
    //! The context API provides a mechanism for components to borrow state from other components higher in the tree.
    //! By combining the Context API and the Subscription API, we can craft ergonomic global state management systems.
    //!
    //! This API is inherently dangerous because we could easily cause UB by allowing &T and &mut T to exist at the same time.
    //! To prevent this, we expose the RemoteState<T> and RemoteLock<T> types which act as a form of reverse borrowing.
    //! This is very similar to RwLock, except that RemoteState is copy-able. Unlike RwLock, derefing RemoteState can
    //! cause panics if the pointer is null. In essence, we sacrifice the panic protection for ergonomics, but arrive at
    //! a similar end result.
    //!
    //! Instead of placing the onus on the receiver of the data to use it properly, we wrap the source object in a
    //! "shield" where gaining &mut access can only be done if no active StateGuards are open. This would fail and indicate
    //! a failure of implementation.
    //!
    //!
    use super::*;

    use std::{marker::PhantomPinned, ops::Deref};

    pub struct RemoteState<T> {
        inner: *const T,
    }
    impl<T> Copy for RemoteState<T> {}

    impl<T> Clone for RemoteState<T> {
        fn clone(&self) -> Self {
            Self { inner: self.inner }
        }
    }

    static DEREF_ERR_MSG: &'static str = r#"""
[ERROR]
This state management implementation is faulty. Report an issue on whatever implementation is using this.
Context should *never* be dangling!. If a Context is torn down, so should anything that references it.
"""#;

    impl<T> Deref for RemoteState<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            // todo!
            // Try to borrow the underlying context manager, register this borrow with the manager as a "weak" subscriber.
            // This will prevent the panic and ensure the pointer still exists.
            // For now, just get an immutable reference to the underlying context data.
            //
            // It's important to note that ContextGuard is not a public API, and can only be made from UseContext.
            // This guard should only be used in components, and never stored in hooks
            unsafe {
                match self.inner.as_ref() {
                    Some(ptr) => ptr,
                    None => panic!(DEREF_ERR_MSG),
                }
            }
        }
    }

    impl<'a> super::Context<'a> {
        // impl<'a, P> super::Context<'a, P> {
        pub fn use_context<I, O>(&'a self, narrow: impl Fn(&'_ I) -> &'_ O) -> RemoteState<O> {
            todo!()
        }
    }

    /// # SAFETY ALERT
    ///
    /// The underlying context mechanism relies on mutating &mut T while &T is held by components in the tree.
    /// By definition, this is UB. Therefore, implementing use_context should be done with upmost care to invalidate and
    /// prevent any code where &T is still being held after &mut T has been taken and T has been mutated.
    ///
    /// While mutating &mut T while &T is captured by listeners, we can do any of:
    ///     1) Prevent those listeners from being called and avoid "producing" UB values
    ///     2) Delete instances of closures where &T is captured before &mut T is taken
    ///     3) Make clones of T to preserve the original &T.
    ///     4) Disable any &T remotely (like RwLock, RefCell, etc)
    ///
    /// To guarantee safe usage of state management solutions, we provide Dioxus-Reducer and Dioxus-Dataflow built on the
    /// SafeContext API. This should provide as an example of how to implement context safely for 3rd party state management.
    ///
    /// It's important to recognize that while safety is a top concern for Dioxus, ergonomics do take prescendence.
    /// Contrasting with the JS ecosystem, Rust is faster, but actually "less safe". JS is, by default, a "safe" language.
    /// However, it does not protect you against data races: the primary concern for 3rd party implementers of Context.
    ///
    /// We guarantee that any &T will remain consistent throughout the life of the Virtual Dom and that
    /// &T is owned by components owned by the VirtualDom. Therefore, it is impossible for &T to:
    /// - be dangling or unaligned
    /// - produce an invalid value
    /// - produce uninitialized memory
    ///
    /// The only UB that is left to the implementer to prevent are Data Races.
    ///
    /// Here's a strategy that is UB:
    /// 1. &T is handed out via use_context
    /// 2. an event is reduced against the state
    /// 3. An &mut T is taken
    /// 4. &mut T is mutated.
    ///
    /// Now, any closures that caputed &T are subject to a data race where they might have skipped checks and UB
    /// *will* affect the program.
    ///
    /// Here's a strategy that's not UB (implemented by SafeContext):
    /// 1. ContextGuard<T> is handed out via use_context.
    /// 2. An event is reduced against the state.
    /// 3. The state is cloned.
    /// 4. All subfield selectors are evaluated and then diffed with the original.
    /// 5. Fields that have changed have their ContextGuard poisoned, revoking their ability to take &T.a.
    /// 6. The affected fields of Context are mutated.
    /// 7. Scopes with poisoned guards are regenerated so they can take &T.a again, calling their lifecycle.
    ///
    /// In essence, we've built a "partial borrowing" mechanism for Context objects.
    ///
    /// =================
    ///       nb
    /// =================
    /// If you want to build a state management API directly and deal with all the unsafe and UB, we provide
    /// `use_context_unchecked` with all the stability with *no* guarantess of Data Race protection. You're on
    /// your own to not affect user applications.
    ///
    /// - Dioxus reducer is built on the safe API and provides a useful but slightly limited API.
    /// - Dioxus Dataflow is built on the unsafe API and provides an even snazzier API than Dioxus Reducer.    
    fn blah() {}
}
