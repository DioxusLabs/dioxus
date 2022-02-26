use dioxus_core::{ScopeState, TaskId};
use std::{any::Any, cell::Cell, future::Future};

use crate::UseFutureDep;

/// A hook that provides a future that executes after the hooks have been applied
///
/// Whenever the hooks dependencies change, the future will be re-evaluated.
/// If a future is pending when the dependencies change, the previous future
/// will be allowed to continue
///
/// - dependencies: a tuple of references to values that are PartialEq + Clone
pub fn use_effect<'a, T: 'static, F: Future<Output = T> + 'static, D: UseFutureDep>(
    cx: &'a ScopeState,
    dependencies: D,
    future: impl FnOnce(D::Out) -> F,
) -> &'a UseEffect {
    let state = cx.use_hook(move |_| UseEffect {
        needs_regen: Cell::new(true),
        task: Cell::new(None),
        dependencies: Vec::new(),
    });

    if dependencies.clone().apply(&mut state.dependencies) || state.needs_regen.get() {
        // We don't need regen anymore
        state.needs_regen.set(false);

        // Create the new future
        let fut = future(dependencies.out());

        state.task.set(Some(cx.push_future(async move {
            fut.await;
        })));
    }

    state
}

pub struct UseEffect {
    needs_regen: Cell<bool>,
    task: Cell<Option<TaskId>>,
    dependencies: Vec<Box<dyn Any>>,
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
            use_effect(&cx, (), |_| async move {
                //
            });

            // runs when a is changed
            use_effect(&cx, (&cx.props.a,), |(a,)| async move {
                //
            });

            // runs when a or b is changed
            use_effect(&cx, (&cx.props.a, &cx.props.b), |(a, b)| async move {
                //
            });

            None
        }
    }
}
