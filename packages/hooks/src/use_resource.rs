#![allow(missing_docs)]

use crate::{use_callback, use_signal, UseCallback};
use dioxus_core::prelude::*;
use dioxus_core::{
    prelude::{spawn, use_hook},
    Task,
};
use dioxus_signals::*;
use futures_util::{future, pin_mut, FutureExt, StreamExt};
use std::ops::Deref;
use std::{cell::Cell, future::Future, rc::Rc};

/// A memo that resolves to a value asynchronously.
/// Similar to `use_future` but `use_resource` returns a value.
/// See [`Resource`] for more details.
/// ```rust
///fn app() -> Element {
///    let country = use_signal(|| WeatherLocation {
///        city: "Berlin".to_string(),
///        country: "Germany".to_string(),
///        coordinates: (52.5244, 13.4105)
///    });
///
///    // Because the resource's future subscribes to `country` by reading it (`country.read()`),
///    // everytime `country` changes the resource's future will run again and thus provide a new value.
///    let current_weather = use_resource(move || async move { get_weather(&country.read().clone()).await });
///    
///    rsx! {
///        // the value of the resource can be polled to
///        // conditionally render elements based off if it's future
///        // finished (Some(Ok(_)), errored Some(Err(_)),
///        // or is still running (None)
///        match current_weather.value() {
///            Some(Ok(weather)) => WeatherElement { weather },
///            Some(Err(e)) => p { "Loading weather failed, {e}" }
///            None =>  p { "Loading..." }
///        }
///    }
///}
/// ```
#[must_use = "Consider using `cx.spawn` to run a future without reading its value"]
pub fn use_resource<T, F>(future: impl Fn() -> F + 'static) -> Resource<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let mut value = use_signal(|| None);
    let mut state = use_signal(|| UseResourceState::Pending);
    let (rc, changed) = use_hook(|| {
        let (rc, changed) = ReactiveContext::new();
        (rc, Rc::new(Cell::new(Some(changed))))
    });

    let cb = use_callback(move || {
        // Create the user's task
        #[allow(clippy::redundant_closure)]
        let fut = rc.run_in(|| future());

        // Spawn a wrapper task that polls the innner future and watch its dependencies
        spawn(async move {
            // move the future here and pin it so we can poll it
            let fut = fut;
            pin_mut!(fut);

            // Run each poll in the context of the reactive scope
            // This ensures the scope is properly subscribed to the future's dependencies
            let res = future::poll_fn(|cx| rc.run_in(|| fut.poll_unpin(cx))).await;

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

#[allow(unused)]
pub struct Resource<T: 'static> {
    value: Signal<Option<T>>,
    task: Signal<Task>,
    state: Signal<UseResourceState>,
    callback: UseCallback<Task>,
}

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
    /// Adds an explicit dependency to the resource. If the dependency changes, the resource's future will rerun.
    ///
    /// Signals will automatically be added as dependencies, so you don't need to call this method for them.
    ///
    /// NOTE: You must follow the rules of hooks when calling this method.
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// # async fn sleep(delay: u32) {}
    ///
    /// #[component]
    /// fn Comp(delay: u32) -> Element {
    ///     // Because the resource subscribes to `delay` by adding it as a dependency, the resource's future will rerun every time `delay` changes.
    ///     let current_weather = use_resource(move || async move {
    ///         sleep(delay).await;
    ///         "Sunny"
    ///     })
    ///     .use_dependencies((&delay,));
    ///
    ///     rsx! {
    ///         // the value of the resource can be polled to
    ///         // conditionally render elements based off if it's future
    ///         // finished (Some(Ok(_)), errored Some(Err(_)),
    ///         // or is still running (None)
    ///         match &*current_weather.read_unchecked() {
    ///             Some(weather) => rsx! { "{weather}" },
    ///             None =>  rsx! { p { "Loading..." } }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn use_dependencies(mut self, dependency: impl Dependency) -> Self {
        let mut dependencies_signal = use_signal(|| dependency.out());
        let changed = { dependency.changed(&*dependencies_signal.read()) };
        if changed {
            dependencies_signal.set(dependency.out());
            self.restart();
        }
        self
    }

    /// Restart the resource's future.
    ///
    /// Will not cancel the previous future, but will ignore any values that it
    /// generates.
    pub fn restart(&mut self) {
        self.task.write().cancel();
        let new_task = self.callback.call();
        self.task.set(new_task);
    }

    /// Forcefully cancel the resource's future.
    pub fn cancel(&mut self) {
        self.state.set(UseResourceState::Stopped);
        self.task.write().cancel();
    }

    /// Pause the resource's future.
    pub fn pause(&mut self) {
        self.state.set(UseResourceState::Paused);
        self.task.write().pause();
    }

    /// Resume the resource's future.
    pub fn resume(&mut self) {
        if self.finished() {
            return;
        }

        self.state.set(UseResourceState::Pending);
        self.task.write().resume();
    }

    /// Clear the resource's value.
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
    pub fn finished(&self) -> bool {
        matches!(
            *self.state.peek(),
            UseResourceState::Ready | UseResourceState::Stopped
        )
    }

    /// Get the current state of the resource's future.
    pub fn state(&self) -> ReadOnlySignal<UseResourceState> {
        self.state.into()
    }

    /// Get the current value of the resource's future.
    pub fn value(&self) -> ReadOnlySignal<Option<T>> {
        self.value.into()
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
