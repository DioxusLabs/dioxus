#![allow(missing_docs)]
use dioxus_core::{ScopeState, TaskId};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    future::{Future, IntoFuture},
    rc::Rc,
    sync::Arc,
};

/// A future that resolves to a value.
///
/// This runs the future only once - though the future may be regenerated
/// through the [`UseFuture::restart`] method.
///
/// This is commonly used for components that cannot be rendered until some
/// asynchronous operation has completed.
///
/// Whenever the hooks dependencies change, the future will be re-evaluated.
/// If a future is pending when the dependencies change, the previous future
/// will be allowed to continue
///
/// - dependencies: a tuple of references to values that are PartialEq + Clone
pub fn use_future<T, F, D>(
    cx: &ScopeState,
    dependencies: D,
    future: impl FnOnce(D::Out) -> F,
) -> &UseFuture<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
    D: UseFutureDep,
{
    let state = cx.use_hook(move || UseFuture {
        update: cx.schedule_update(),
        needs_regen: Cell::new(true),
        values: Default::default(),
        task: Cell::new(None),
        dependencies: Vec::new(),
        waker: Default::default(),
    });

    *state.waker.borrow_mut() = None;

    if dependencies.clone().apply(&mut state.dependencies) || state.needs_regen.get() {
        // We don't need regen anymore
        state.needs_regen.set(false);

        // Create the new future
        let fut = future(dependencies.out());

        // Clone in our cells
        let values = state.values.clone();
        let schedule_update = state.update.clone();
        let waker = state.waker.clone();

        // Cancel the current future
        if let Some(current) = state.task.take() {
            cx.remove_future(current);
        }

        state.task.set(Some(cx.push_future(async move {
            let res = fut.await;
            values.borrow_mut().push(Box::leak(Box::new(res)));

            // if there's a waker, we dont re-render the component. Instead we just progress that future
            match waker.borrow().as_ref() {
                Some(waker) => waker.wake_by_ref(),
                None => schedule_update(),
            }
        })));
    }

    state
}

pub enum FutureState<'a, T> {
    Pending,
    Complete(&'a T),
    Regenerating(&'a T), // the old value
}

pub struct UseFuture<T> {
    update: Arc<dyn Fn()>,
    needs_regen: Cell<bool>,
    task: Cell<Option<TaskId>>,
    dependencies: Vec<Box<dyn Any>>,
    waker: Rc<RefCell<Option<std::task::Waker>>>,
    values: Rc<RefCell<Vec<*mut T>>>,
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
            cx.remove_future(task);
        }
    }

    // clears the value in the future slot without starting the future over
    pub fn clear(&self) -> Option<T> {
        todo!()
        // (self.update)();
        // self.slot.replace(None)
    }

    // Manually set the value in the future slot without starting the future over
    pub fn set(&self, _new_value: T) {
        // self.slot.set(Some(new_value));
        // self.needs_regen.set(true);
        // (self.update)();
        todo!()
    }

    /// Return any value, even old values if the future has not yet resolved.
    ///
    /// If the future has never completed, the returned value will be `None`.
    pub fn value(&self) -> Option<&T> {
        self.values
            .borrow_mut()
            .last()
            .cloned()
            .map(|x| unsafe { &*x })
    }

    /// Get the ID of the future in Dioxus' internal scheduler
    pub fn task(&self) -> Option<TaskId> {
        self.task.get()
    }

    /// Get the current state of the future.
    pub fn state(&self) -> UseFutureState<T> {
        match (&self.task.get(), &self.value()) {
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

impl<'a, T> IntoFuture for &'a UseFuture<T> {
    type Output = &'a T;
    type IntoFuture = UseFutureAwait<'a, T>;
    fn into_future(self) -> Self::IntoFuture {
        UseFutureAwait { hook: self }
    }
}

pub struct UseFutureAwait<'a, T> {
    hook: &'a UseFuture<T>,
}

impl<'a, T> Future for UseFutureAwait<'a, T> {
    type Output = &'a T;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.hook.values.borrow_mut().last().cloned() {
            Some(value) => std::task::Poll::Ready(unsafe { &*value }),
            None => {
                self.hook.waker.replace(Some(cx.waker().clone()));
                std::task::Poll::Pending
            }
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
impl_dep!(A = a, B = b, C = c, D = d, E = e, F = f, G = g,);
impl_dep!(A = a, B = b, C = c, D = d, E = e, F = f, G = g, H = h,);

/// A helper macro that merges uses the closure syntax to elaborate the dependency array
#[macro_export]
macro_rules! use_future {
    ($cx:ident, || $($rest:tt)*) => { use_future( $cx, (), |_| $($rest)* ) };
    ($cx:ident, | $($args:tt),* | $($rest:tt)*) => {
        use_future(
            $cx,
            ($($args),*),
            |($($args),*)| $($rest)*
        )
    };
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

        async fn app(cx: Scope<'_, MyProps>) -> Element {
            // should only ever run once
            let fut = use_future(cx, (), |_| async move {});

            // runs when a is changed
            let fut = use_future(cx, (&cx.props.a,), |(a,)| async move {});

            // runs when a or b is changed
            let fut = use_future(cx, (&cx.props.a, &cx.props.b), |(a, b)| async move { 123 });

            let a = use_future!(cx, || async move {
                // do the thing!
            });

            let b = &123;
            let c = &123;

            let a = use_future!(cx, |b, c| async move {
                let a = b + c;
                let blah = "asd";
            });

            let g2 = a.await;

            let g = fut.await;

            todo!()
        }
    }
}
