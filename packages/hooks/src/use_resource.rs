#![allow(missing_docs)]

use crate::{use_callback, use_signal, UseCallback};
use dioxus_core::prelude::*;
use dioxus_signals::*;
use futures_util::{future, pin_mut, FutureExt, StreamExt};
use std::ops::Deref;
use std::{cell::Cell, future::Future, rc::Rc};

#[doc = include_str!("../docs/use_resource.md")]
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[doc = include_str!("../docs/moving_state_around.md")]
#[doc(alias = "use_async_memo")]
#[doc(alias = "use_memo_async")]
#[must_use = "Consider using `cx.spawn` to run a future without reading its value"]
#[track_caller]
pub fn use_resource<T, F>(mut future: impl FnMut() -> F + 'static) -> Resource<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let location = std::panic::Location::caller();

    let mut value = use_signal(|| None);
    let mut state = use_signal(|| UseResourceState::Pending);
    let (rc, changed) = use_hook(|| {
        let (rc, changed) = ReactiveContext::new_with_origin(location);
        (rc, Rc::new(Cell::new(Some(changed))))
    });

    let cb = use_callback(move || {
        // Create the user's task
        let fut = rc.reset_and_run_in(&mut future);

        // Spawn a wrapper task that polls the inner future and watch its dependencies
        spawn(async move {
            // move the future here and pin it so we can poll it
            let fut = fut;
            pin_mut!(fut);

            // Run each poll in the context of the reactive scope
            // This ensures the scope is properly subscribed to the future's dependencies
            let res = future::poll_fn(|cx| {
                rc.run_in(|| {
                    tracing::trace_span!("polling resource", location = %location)
                        .in_scope(|| fut.poll_unpin(cx))
                })
            })
            .await;

            // Set the value and state
            state.set(UseResourceState::Ready);
            value.set(Some(res));
        })
    });

    let mut task = use_hook(|| Signal::new(cb()));

    use_hook(|| {
        let mut changed = changed.take().unwrap();
        spawn(async move {
            loop {
                // Wait for the dependencies to change
                let _ = changed.next().await;

                // Stop the old task
                task.write().cancel();

                // Start a new task
                task.set(cb());
            }
        })
    });

    Resource {
        task,
        value,
        state,
        callback: cb,
    }
}

/// A handle to a reactive future spawned with [`use_resource`] that can be used to modify or read the result of the future.
///
/// ## Example
///
/// Reading the result of a resource:
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use std::time::Duration;
/// fn App() -> Element {
///     let mut revision = use_signal(|| "1d03b42");
///     let mut resource = use_resource(move || async move {
///         // This will run every time the revision signal changes because we read the count inside the future
///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
///     });
///
///     // Since our resource may not be ready yet, the value is an Option. Our request may also fail, so the get function returns a Result
///     // The complete type we need to match is `Option<Result<String, reqwest::Error>>`
///     // We can use `read_unchecked` to keep our matching code in one statement while avoiding a temporary variable error (this is still completely safe because dioxus checks the borrows at runtime)
///     match &*resource.read_unchecked() {
///         Some(Ok(value)) => rsx! { "{value:?}" },
///         Some(Err(err)) => rsx! { "Error: {err}" },
///         None => rsx! { "Loading..." },
///     }
/// }
/// ```
#[derive(Debug)]
pub struct Resource<T: 'static> {
    value: Signal<Option<T>>,
    task: Signal<Task>,
    state: Signal<UseResourceState>,
    callback: UseCallback<Task>,
}

impl<T> PartialEq for Resource<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.state == other.state
            && self.task == other.task
            && self.callback == other.callback
    }
}

impl<T> Clone for Resource<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Resource<T> {}

/// A signal that represents the state of the resource
// we might add more states (panicked, etc)
#[derive(Clone, Copy, PartialEq, Hash, Eq, Debug)]
pub enum UseResourceState {
    /// The resource's future is still running
    Pending,

    /// The resource's future has been forcefully stopped
    Stopped,

    /// The resource's future has been paused, tempoarily
    Paused,

    /// The resource's future has completed
    Ready,
}

impl<T> Resource<T> {
    /// Restart the resource's future.
    ///
    /// This will cancel the current future and start a new one.
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     rsx! {
    ///         button {
    ///             // We can get a signal with the value of the resource with the `value` method
    ///             onclick: move |_| resource.restart(),
    ///             "Restart resource"
    ///         }
    ///         "{resource:?}"
    ///     }
    /// }
    /// ```
    pub fn restart(&mut self) {
        self.task.write().cancel();
        let new_task = self.callback.call();
        self.task.set(new_task);
    }

    /// Forcefully cancel the resource's future.
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     rsx! {
    ///         button {
    ///             // We can cancel the resource before it finishes with the `cancel` method
    ///             onclick: move |_| resource.cancel(),
    ///             "Cancel resource"
    ///         }
    ///         "{resource:?}"
    ///     }
    /// }
    /// ```
    pub fn cancel(&mut self) {
        self.state.set(UseResourceState::Stopped);
        self.task.write().cancel();
    }

    /// Pause the resource's future.
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     rsx! {
    ///         button {
    ///             // We can pause the future with the `pause` method
    ///             onclick: move |_| resource.pause(),
    ///             "Pause"
    ///         }
    ///         button {
    ///             // And resume it with the `resume` method
    ///             onclick: move |_| resource.resume(),
    ///             "Resume"
    ///         }
    ///         "{resource:?}"
    ///     }
    /// }
    /// ```
    pub fn pause(&mut self) {
        self.state.set(UseResourceState::Paused);
        self.task.write().pause();
    }

    /// Resume the resource's future.
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     rsx! {
    ///         button {
    ///             // We can pause the future with the `pause` method
    ///             onclick: move |_| resource.pause(),
    ///             "Pause"
    ///         }
    ///         button {
    ///             // And resume it with the `resume` method
    ///             onclick: move |_| resource.resume(),
    ///             "Resume"
    ///         }
    ///         "{resource:?}"
    ///     }
    /// }
    /// ```
    pub fn resume(&mut self) {
        if self.finished() {
            return;
        }

        self.state.set(UseResourceState::Pending);
        self.task.write().resume();
    }

    /// Clear the resource's value. This will just reset the value. It will not modify any running tasks.
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     rsx! {
    ///         button {
    ///             // We clear the value without modifying any running tasks with the `clear` method
    ///             onclick: move |_| resource.clear(),
    ///             "Clear"
    ///         }
    ///         "{resource:?}"
    ///     }
    /// }
    /// ```
    pub fn clear(&mut self) {
        self.value.write().take();
    }

    /// Get a handle to the inner task backing this resource
    /// Modify the task through this handle will cause inconsistent state
    pub fn task(&self) -> Task {
        self.task.cloned()
    }

    /// Is the resource's future currently finished running?
    ///
    /// Reading this does not subscribe to the future's state
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     // We can use the `finished` method to check if the future is finished
    ///     if resource.finished() {
    ///         rsx! {
    ///             "The resource is finished"
    ///         }
    ///     } else {
    ///         rsx! {
    ///             "The resource is still running"
    ///         }
    ///     }
    /// }
    /// ```
    pub fn finished(&self) -> bool {
        matches!(
            *self.state.peek(),
            UseResourceState::Ready | UseResourceState::Stopped
        )
    }

    /// Get the current state of the resource's future. This method returns a [`ReadOnlySignal`] which can be read to get the current state of the resource or passed to other hooks and components.
    ///
    /// ## Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     // We can read the current state of the future with the `state` method
    ///     match resource.state().cloned() {
    ///         UseResourceState::Pending => rsx! {
    ///             "The resource is still pending"
    ///         },
    ///         UseResourceState::Paused => rsx! {
    ///             "The resource has been paused"
    ///         },
    ///         UseResourceState::Stopped => rsx! {
    ///             "The resource has been stopped"
    ///         },
    ///         UseResourceState::Ready => rsx! {
    ///             "The resource is ready!"
    ///         },
    ///     }
    /// }
    /// ```
    pub fn state(&self) -> ReadOnlySignal<UseResourceState> {
        self.state.into()
    }

    /// Get the current value of the resource's future.  This method returns a [`ReadOnlySignal`] which can be read to get the current value of the resource or passed to other hooks and components.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use std::time::Duration;
    /// fn App() -> Element {
    ///     let mut revision = use_signal(|| "1d03b42");
    ///     let mut resource = use_resource(move || async move {
    ///         // This will run every time the revision signal changes because we read the count inside the future
    ///         reqwest::get(format!("https://github.com/DioxusLabs/awesome-dioxus/blob/{revision}/awesome.json")).await
    ///     });
    ///
    ///     // We can get a signal with the value of the resource with the `value` method
    ///     let value = resource.value();
    ///
    ///     // Since our resource may not be ready yet, the value is an Option. Our request may also fail, so the get function returns a Result
    ///     // The complete type we need to match is `Option<Result<String, reqwest::Error>>`
    ///     // We can use `read_unchecked` to keep our matching code in one statement while avoiding a temporary variable error (this is still completely safe because dioxus checks the borrows at runtime)
    ///     match &*value.read_unchecked() {
    ///         Some(Ok(value)) => rsx! { "{value:?}" },
    ///         Some(Err(err)) => rsx! { "Error: {err}" },
    ///         None => rsx! { "Loading..." },
    ///     }
    /// }
    /// ```
    pub fn value(&self) -> ReadOnlySignal<Option<T>> {
        self.value.into()
    }

    /// Suspend the resource's future and only continue rendering when the future is ready
    pub fn suspend(&self) -> std::result::Result<MappedSignal<T>, RenderError> {
        match self.state.cloned() {
            UseResourceState::Stopped | UseResourceState::Paused | UseResourceState::Pending => {
                let task = self.task();
                if task.paused() {
                    Ok(self.value.map(|v| v.as_ref().unwrap()))
                } else {
                    Err(RenderError::Suspended(SuspendedFuture::new(task)))
                }
            }
            _ => Ok(self.value.map(|v| v.as_ref().unwrap())),
        }
    }
}

impl<T> From<Resource<T>> for ReadOnlySignal<Option<T>> {
    fn from(val: Resource<T>) -> Self {
        val.value.into()
    }
}

impl<T> Readable for Resource<T> {
    type Target = Option<T>;
    type Storage = UnsyncStorage;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.value.try_read_unchecked()
    }

    #[track_caller]
    fn peek_unchecked(&self) -> ReadableRef<'static, Self> {
        self.value.peek_unchecked()
    }
}

impl<T> IntoAttributeValue for Resource<T>
where
    T: Clone + IntoAttributeValue,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T> IntoDynNode for Resource<T>
where
    T: Clone + IntoDynNode,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self().into_dyn_node()
    }
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl<T: Clone> Deref for Resource<T> {
    type Target = dyn Fn() -> Option<T>;

    fn deref(&self) -> &Self::Target {
        Readable::deref_impl(self)
    }
}
