//! Context API
//!
//! The context API provides a mechanism for components to grab
//!
//!
//!

use std::marker::PhantomPinned;

/// Any item that works with app
pub trait AppContext {}

#[derive(Copy, Clone)]
pub struct ContextGuard<'a, T> {
    inner: *mut T,
    _p: std::marker::PhantomData<&'a ()>,
}

impl<'a, PropType> super::context::Context<'a, PropType> {
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
    pub fn use_context<C: AppContext>(&'a self) -> C {
        todo!()
    }

    pub unsafe fn use_context_unchecked<C: AppContext>() {}
}

struct SafeContext<T> {
    value: T,

    // This context is pinned
    _pinned: PhantomPinned,
}
