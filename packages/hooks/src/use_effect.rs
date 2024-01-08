use dioxus_core::{ScopeState, TaskId};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    future::Future,
    rc::Rc,
};

use crate::UseFutureDep;

/// A hook that provides a future that executes after the hooks have been applied.
///
/// Whenever the hooks dependencies change, the future will be re-evaluated.
/// If a future is pending when the dependencies change, the previous future
/// will be allowed to continue.
///
/// **Note:** If your dependency list is always empty, use [`use_on_create`](crate::use_on_create).
///
/// ## Arguments
///
/// - `dependencies`: a tuple of references to values that are `PartialEq` + `Clone`.
/// - `future`: a closure that takes the `dependencies` as arguments and returns a `'static` future. That future may return nothing or a closure that will be executed when the dependencies change to clean up the effect.
///
/// ## Examples
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// #[component]
/// fn Profile(cx: Scope, id: usize) -> Element {
///     let name = use_state(cx, || None);
///
///     // Only fetch the user data when the id changes.
///     use_effect(cx, (id,), |(id,)| {
///         to_owned![name];
///         async move {
///             let user = fetch_user(id).await;
///             name.set(user.name);
///         }
///     });
///
///     // Only fetch the user data when the id changes.
///     use_effect(cx, (id,), |(id,)| {
///         to_owned![name];
///         async move {
///             let user = fetch_user(id).await;
///             name.set(user.name);
///             move || println!("Cleaning up from {}", id)
///         }
///     });
///
///     let name = name.get().clone().unwrap_or("Loading...".to_string());
///
///     render!(
///         p { "{name}" }
///     )
/// }
///
/// #[component]
/// fn App(cx: Scope) -> Element {
///     render!(Profile { id: 0 })
/// }
/// ```
pub fn use_effect<T, R, D>(cx: &ScopeState, dependencies: D, future: impl FnOnce(D::Out) -> R)
where
    D: UseFutureDep,
    R: UseEffectReturn<T>,
{
    struct UseEffect {
        needs_regen: bool,
        task: Cell<Option<TaskId>>,
        dependencies: Vec<Box<dyn Any>>,
        cleanup: UseEffectCleanup,
    }

    impl Drop for UseEffect {
        fn drop(&mut self) {
            if let Some(cleanup) = self.cleanup.borrow_mut().take() {
                cleanup();
            }
        }
    }

    let state = cx.use_hook(move || UseEffect {
        needs_regen: true,
        task: Cell::new(None),
        dependencies: Vec::new(),
        cleanup: Rc::new(RefCell::new(None)),
    });

    if dependencies.clone().apply(&mut state.dependencies) || state.needs_regen {
        // Call the cleanup function if it exists
        if let Some(cleanup) = state.cleanup.borrow_mut().take() {
            cleanup();
        }

        // We don't need regen anymore
        state.needs_regen = false;

        // Create the new future
        let return_value = future(dependencies.out());

        let task = return_value.apply(state.cleanup.clone(), cx);
        state.task.set(Some(task));
    }
}

type UseEffectCleanup = Rc<RefCell<Option<Box<dyn FnOnce()>>>>;

/// Something that can be returned from a `use_effect` hook.
pub trait UseEffectReturn<T> {
    fn apply(self, oncleanup: UseEffectCleanup, cx: &ScopeState) -> TaskId;
}

impl<T> UseEffectReturn<()> for T
where
    T: Future<Output = ()> + 'static,
{
    fn apply(self, _: UseEffectCleanup, cx: &ScopeState) -> TaskId {
        cx.push_future(self)
    }
}

#[doc(hidden)]
pub struct CleanupFutureMarker;
impl<T, F> UseEffectReturn<CleanupFutureMarker> for T
where
    T: Future<Output = F> + 'static,
    F: FnOnce() + 'static,
{
    fn apply(self, oncleanup: UseEffectCleanup, cx: &ScopeState) -> TaskId {
        cx.push_future(async move {
            let cleanup = self.await;
            *oncleanup.borrow_mut() = Some(Box::new(cleanup) as Box<dyn FnOnce()>);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused)]
    #[test]
    fn test_use_future() {
        use dioxus_core::prelude::*;

        struct MyProps {
            a: String,
            b: i32,
            c: i32,
            d: i32,
            e: i32,
        }

        fn app(cx: Scope<MyProps>) -> Element {
            // should only ever run once
            use_effect(cx, (), |_| async move {
                //
            });

            // runs when a is changed
            use_effect(cx, (&cx.props.a,), |(a,)| async move {
                //
            });

            // runs when a or b is changed
            use_effect(cx, (&cx.props.a, &cx.props.b), |(a, b)| async move {
                //
            });

            todo!()
        }
    }
}
