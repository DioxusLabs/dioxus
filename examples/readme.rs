//! Example: README.md showcase
//!
//! The example from the README.md.

use std::{any::Any, cell::RefCell, marker::PhantomData, pin::Pin};

use dioxus::prelude::{use_state, Element, Scoped};
// use dioxus::prelude::*;
use futures_util::{future::abortable, Future};

fn main() {
    // dioxus_desktop::launch(app);
}

fn app(cx: Scope) {
    // let name = use_state(cx, || "world".to_string());

    let name = cx.use_hook(|| "asdasd".to_string());

    cx.spawn(async {
        println!("Hello, world! {name}");
    });

    use_future(cx, async move {
        println!("Hello, world! {name}");
    });

    todo!()
}

pub fn use_future<'a>(cx: Scope<'a>, f: impl Future<Output = ()> + 'a) {
    todo!()
}

pub fn create_ref<T: 'static>(cx: Scope, value: T) -> &T {
    todo!()
    // cx.raw.arena.alloc(value)
}

pub fn spawn_local_scoped<'a>(cx: Scope<'a>, f: impl Future<Output = ()> + 'a) {
    let boxed: Pin<Box<dyn Future<Output = ()> + 'a>> = Box::pin(f);
    // SAFETY: We are just transmuting the lifetime here so that we can spawn the future.
    // This is safe because we wrap the future in an `Abortable` future which will be
    // immediately aborted once the reactive scope is dropped.
    let extended: Pin<Box<dyn Future<Output = ()> + 'static>> =
        unsafe { std::mem::transmute(boxed) };

    let (abortable, handle) = abortable(extended);

    tokio::task::spawn_local(abortable);
}

// let mut count = use_state(&cx, || 0);
// let hook = cx.raw.use_hook(|| 10);

// cx.render(rsx! {
//     h1 { "High-Five counter: {count}" }
//     button { onclick: move |_| count += 1, "Up high!" }
//     button { onclick: move |_| count -= 1, "Down low!" }
// })

// count.set(10);
// let r = name.as_bytes();

struct ScopeRaw<'a> {
    inner: RefCell<ScopeInner<'a>>,
    /// A pointer to the parent scope.
    /// # Safety
    /// The parent scope does not actually have the right lifetime.
    parent: Option<*const ScopeRaw<'a>>,
}

/// A wrapper type around a lifetime that forces the lifetime to be invariant.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct InvariantLifetime<'id>(PhantomData<&'id mut &'id ()>);

/// Internal representation for [`Scope`]. This allows only using a single top-level [`RefCell`]
/// instead of a [`RefCell`] for every field.
#[derive(Default)]
struct ScopeInner<'a> {
    /// The depth of the current scope. The root scope has a depth of 0. Any child scopes have a
    /// depth of N + 1 where N is the depth of the parent scope.
    depth: u32,
    /// If this is true, this will prevent the scope from being dropped.
    /// This is set when an effect is running to prevent an use-after-free.
    lock_drop: bool,
    // Make sure that 'a is invariant.
    _phantom: InvariantLifetime<'a>,
}

// impl<'a> ScopeInner<'a> {
//     fn alloc(&'a self, val: T) -> &'a T {
//         todo!()
//     }
// }

#[derive(Clone, Copy)]
pub struct BoundedScope<'a> {
    raw: &'a ScopeRaw<'a>,
    // /// `&'b` for covariance!
    // _phantom: PhantomData<&'b ()>,
}

impl<'a> BoundedScope<'a> {
    fn alloc<T: 'static>(self, value: T) -> &'a T {
        todo!()
    }
    fn use_hook<T: 'static>(self, value: impl FnOnce() -> T) -> &'a T {
        todo!()
    }

    fn spawn(self, f: impl Future<Output = ()> + 'a) {
        spawn_local_scoped(self, f)
    }
}

impl Drop for ScopeRaw<'_> {
    fn drop(&mut self) {
        todo!()
        // // SAFETY: scope cannot be dropped while it is borrowed inside closure.
        // unsafe { self.dispose() };
    }
}

/// A type-alias for [`BoundedScope`] where both lifetimes are the same.
pub type Scope<'a> = BoundedScope<'a>;
