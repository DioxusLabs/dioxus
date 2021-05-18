use crate::{innerlude::*, nodebuilder::IntoDomTree};
use crate::{nodebuilder::LazyNodes, nodes::VNode};
use bumpalo::Bump;
use std::{cell::RefCell, future::Future, ops::Deref, pin::Pin, rc::Rc, sync::atomic::AtomicUsize};

/// Context API
///
/// The context API provides a mechanism for components to borrow state from other components higher in the tree.
/// By combining the Context API and the Subscription API, we can craft ergonomic global state management systems.
///
/// This API is inherently dangerous because we could easily cause UB by allowing &T and &mut T to exist at the same time.
/// To prevent this, we expose the RemoteState<T> and RemoteLock<T> types which act as a form of reverse borrowing.
/// This is very similar to RwLock, except that RemoteState is copy-able. Unlike RwLock, derefing RemoteState can
/// cause panics if the pointer is null. In essence, we sacrifice the panic protection for ergonomics, but arrive at
/// a similar end result.
///
/// Instead of placing the onus on the receiver of the data to use it properly, we wrap the source object in a
/// "shield" where gaining &mut access can only be done if no active StateGuards are open. This would fail and indicate
/// a failure of implementation.

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

// impl<'a> crate::virtual_dom::Context<'a> {
//     // impl<'a, P> super::Context<'a, P> {
//     pub fn use_context<I, O>(&'a self, _narrow: impl Fn(&'_ I) -> &'_ O) -> RemoteState<O> {
//         todo!()
//     }

//     pub fn create_context<T: 'static>(&self, creator: impl FnOnce() -> T) {}
// }

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
fn _blah() {}
