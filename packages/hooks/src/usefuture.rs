use dioxus_core::{ScopeState, TaskId};
use std::{any::Any, cell::Cell, future::Future, rc::Rc, sync::Arc};

/// A hook that provides a future that will resolve to a value.
///
/// Whenever the hooks dependencies change, the future will be re-evaluated.
/// If a future is pending when the dependencies change, the previous future
/// will be allowed to continue
///
///
/// - dependencies: a tuple of references to values that are PartialEq + Clone
pub fn use_future<'a, T: 'static, F: Future<Output = T> + 'static, D: UseFutureDep>(
    cx: &'a ScopeState,
    dependencies: D,
    future: impl FnOnce(D::Out) -> F,
) -> &'a UseFuture<T> {
    let state = cx.use_hook(move |_| UseFuture {
        update: cx.schedule_update(),
        needs_regen: Cell::new(true),
        slot: Rc::new(Cell::new(None)),
        value: None,
        task: Cell::new(None),
        dependencies: Vec::new(),
    });

    if let Some(value) = state.slot.take() {
        state.value = Some(value);
        state.task.set(None);
    }

    if dependencies.clone().apply(&mut state.dependencies) || state.needs_regen.get() {
        // We don't need regen anymore
        state.needs_regen.set(false);

        // Create the new future
        let fut = future(dependencies.out());

        // Clone in our cells
        let slot = state.slot.clone();
        let schedule_update = state.update.clone();

        // Cancel the current future
        if let Some(current) = state.task.take() {
            cx.cancel_future(current);
        }

        state.task.set(Some(cx.push_future(async move {
            let res = fut.await;
            slot.set(Some(res));
            schedule_update();
        })));
    }

    state
}

pub struct UseFuture<T> {
    update: Arc<dyn Fn()>,
    needs_regen: Cell<bool>,
    value: Option<T>,
    slot: Rc<Cell<Option<T>>>,
    task: Cell<Option<TaskId>>,
    dependencies: Vec<Box<dyn Any>>,
}

pub enum UseFutureState<'a, T> {
    Pending,
    Complete(&'a T),
    Reloading(&'a T),
}

impl<T> UseFuture<T> {
    /// Restart the future with new dependencies.
    ///
    /// Will not cancel the previous future, but will ignore any values that it
    /// generates.
    pub fn restart(&self) {
        self.needs_regen.set(true);
        (self.update)();
    }

    /// Forcefully cancel a future
    pub fn cancel(&self, cx: &ScopeState) {
        if let Some(task) = self.task.take() {
            cx.cancel_future(task);
        }
    }

    // clears the value in the future slot without starting the future over
    pub fn clear(&self) -> Option<T> {
        (self.update)();
        self.slot.replace(None)
    }

    // Manually set the value in the future slot without starting the future over
    pub fn set(&self, new_value: T) {
        self.slot.set(Some(new_value));
        self.needs_regen.set(true);
        (self.update)();
    }

    /// Return any value, even old values if the future has not yet resolved.
    ///
    /// If the future has never completed, the returned value will be `None`.
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Get the ID of the future in Dioxus' internal scheduler
    pub fn task(&self) -> Option<TaskId> {
        self.task.get()
    }

    /// Get the current stateof the future.
    pub fn state(&self) -> UseFutureState<T> {
        match (&self.task.get(), &self.value) {
            // If we have a task and an existing value, we're reloading
            (Some(_), Some(val)) => UseFutureState::Reloading(val),

            // no task, but value - we're done
            (None, Some(val)) => UseFutureState::Complete(val),

            // no task, no value - something's wrong? return pending
            (None, None) => UseFutureState::Pending,

            // Task, no value - we're still pending
            (Some(_), None) => UseFutureState::Pending,
        }
    }
}

pub trait UseFutureDep: Sized + Clone {
    type Out;
    fn out(&self) -> Self::Out;
    fn apply(self, state: &mut Vec<Box<dyn Any>>) -> bool;
}

impl UseFutureDep for () {
    type Out = ();
    fn out(&self) -> Self::Out {}
    fn apply(self, _state: &mut Vec<Box<dyn Any>>) -> bool {
        false
    }
}

pub trait Dep: 'static + PartialEq + Clone {}
impl<T> Dep for T where T: 'static + PartialEq + Clone {}

impl<A: Dep> UseFutureDep for &A {
    type Out = A;
    fn out(&self) -> Self::Out {
        (*self).clone()
    }
    fn apply(self, state: &mut Vec<Box<dyn Any>>) -> bool {
        match state.get_mut(0).and_then(|f| f.downcast_mut::<A>()) {
            Some(val) => {
                if *val != *self {
                    *val = self.clone();
                    return true;
                }
            }
            None => {
                state.push(Box::new(self.clone()));
                return true;
            }
        }
        false
    }
}

macro_rules! impl_dep {
    (
        $($el:ident=$name:ident,)*
    ) => {
        impl< $($el),* > UseFutureDep for ($(&$el,)*)
        where
            $(
                $el: Dep
            ),*
        {
            type Out = ($($el,)*);

            fn out(&self) -> Self::Out {
                let ($($name,)*) = self;
                ($((*$name).clone(),)*)
            }

            #[allow(unused)]
            fn apply(self, state: &mut Vec<Box<dyn Any>>) -> bool {
                let ($($name,)*) = self;
                let mut idx = 0;
                let mut needs_regen = false;

                $(
                    match state.get_mut(idx).map(|f| f.downcast_mut::<$el>()).flatten() {
                        Some(val) => {
                            if *val != *$name {
                                *val = $name.clone();
                                needs_regen = true;
                            }
                        }
                        None => {
                            state.push(Box::new($name.clone()));
                            needs_regen = true;
                        }
                    }
                    idx += 1;
                )*

                needs_regen
            }
        }
    };
}

impl_dep!(A = a,);
impl_dep!(A = a, B = b,);
impl_dep!(A = a, B = b, C = c,);
impl_dep!(A = a, B = b, C = c, D = d,);
impl_dep!(A = a, B = b, C = c, D = d, E = e,);
impl_dep!(A = a, B = b, C = c, D = d, E = e, F = f,);

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
            let fut = use_future(&cx, (), |_| async move {
                //
            });

            // runs when a is changed
            let fut = use_future(&cx, (&cx.props.a,), |(a,)| async move {
                //
            });

            // runs when a or b is changed
            let fut = use_future(&cx, (&cx.props.a, &cx.props.b), |(a, b)| async move {
                //
            });

            None
        }
    }
}
