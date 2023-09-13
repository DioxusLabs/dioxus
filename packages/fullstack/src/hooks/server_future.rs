use dioxus::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use std::any::Any;
use std::cell::Cell;
use std::cell::Ref;
use std::cell::RefCell;
use std::fmt::Debug;
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;

/// A future that resolves to a value.
///
/// This runs the future only once - though the future may be regenerated
/// through the [`UseServerFuture::restart`] method.
///
/// This is commonly used for components that cannot be rendered until some
/// asynchronous operation has completed.
///
/// Whenever the hooks dependencies change, the future will be re-evaluated.
/// If a future is pending when the dependencies change, the previous future
/// will be allowed to continue
///
/// - dependencies: a tuple of references to values that are PartialEq + Clone
pub fn use_server_future<T, F, D>(
    cx: &ScopeState,
    dependencies: D,
    future: impl FnOnce(D::Out) -> F,
) -> Option<&UseServerFuture<T>>
where
    T: 'static + Serialize + DeserializeOwned + Debug,
    F: Future<Output = T> + 'static,
    D: UseFutureDep,
{
    let state = cx.use_hook(move || UseServerFuture {
        update: cx.schedule_update(),
        needs_regen: Cell::new(true),
        value: Default::default(),
        task: Cell::new(None),
        dependencies: Vec::new(),
    });

    let first_run = { state.value.borrow().as_ref().is_none() && state.task.get().is_none() };

    #[cfg(not(feature = "ssr"))]
    {
        if first_run {
            match crate::html_storage::deserialize::take_server_data() {
                Some(data) => {
                    tracing::trace!("Loaded {data:?} from server");
                    *state.value.borrow_mut() = Some(Box::new(data));
                    state.needs_regen.set(false);
                    return Some(state);
                }
                None => {
                    tracing::trace!("Failed to load from server... running future");
                }
            };
        }
    }

    if dependencies.clone().apply(&mut state.dependencies) || state.needs_regen.get() {
        // We don't need regen anymore
        state.needs_regen.set(false);

        // Create the new future
        let fut = future(dependencies.out());

        // Clone in our cells
        let value = state.value.clone();
        let schedule_update = state.update.clone();

        // Cancel the current future
        if let Some(current) = state.task.take() {
            cx.remove_future(current);
        }

        state.task.set(Some(cx.push_future(async move {
            let data;
            #[cfg(feature = "ssr")]
            {
                data = fut.await;
                if first_run {
                    if let Err(err) = crate::prelude::server_context().push_html_data(&data) {
                        tracing::error!("Failed to push HTML data: {}", err);
                    };
                }
            }
            #[cfg(not(feature = "ssr"))]
            {
                data = fut.await;
            }
            *value.borrow_mut() = Some(Box::new(data));

            schedule_update();
        })));
    }

    if first_run {
        #[cfg(feature = "ssr")]
        {
            tracing::trace!("Suspending first run of use_server_future");
            cx.suspend();
        }
        None
    } else {
        Some(state)
    }
}

pub struct UseServerFuture<T> {
    update: Arc<dyn Fn()>,
    needs_regen: Cell<bool>,
    task: Cell<Option<TaskId>>,
    dependencies: Vec<Box<dyn Any>>,
    value: Rc<RefCell<Option<Box<T>>>>,
}

impl<T> UseServerFuture<T> {
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

    /// Return any value, even old values if the future has not yet resolved.
    ///
    /// If the future has never completed, the returned value will be `None`.
    pub fn value(&self) -> Ref<'_, T> {
        Ref::map(self.value.borrow(), |v| v.as_deref().unwrap())
    }

    /// Get the ID of the future in Dioxus' internal scheduler
    pub fn task(&self) -> Option<TaskId> {
        self.task.get()
    }

    /// Get the current state of the future.
    pub fn reloading(&self) -> bool {
        self.task.get().is_some()
    }
}
