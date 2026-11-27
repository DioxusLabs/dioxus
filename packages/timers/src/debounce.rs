use crate::{TimeoutHandle, UseTimeout, use_timeout};
use dioxus_hooks::use_signal;
use dioxus_signals::{Signal, WritableExt as _};
use dioxus_vdom::SpawnIfAsync;
use std::time::Duration;

/// The interface for calling a debounce.
///
/// See [`use_debounce`] for more information.
pub struct UseDebounce<Args: 'static> {
    current_handle: Signal<Option<TimeoutHandle>>,
    timeout: UseTimeout<Args>,
}

impl<Args> UseDebounce<Args> {
    /// Start the debounce countdown, resetting it if already started.
    pub fn action(&mut self, args: Args) {
        self.cancel();
        self.current_handle.set(Some(self.timeout.action(args)));
    }

    /// Cancel the debounce action.
    pub fn cancel(&mut self) {
        if let Some(handle) = self.current_handle.take() {
            handle.cancel();
        }
    }
}

impl<Args> Clone for UseDebounce<Args> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<Args> Copy for UseDebounce<Args> {}
impl<Args> PartialEq for UseDebounce<Args> {
    fn eq(&self, other: &Self) -> bool {
        self.current_handle == other.current_handle && self.timeout == other.timeout
    }
}

/// A hook for allowing a function to be called only after a provided [`Duration`] has passed.
///
/// Once the [`UseDebounce::action`] method is called, a timer will start counting down until
/// the callback is ran. If the [`UseDebounce::action`] method is called again, the timer will restart.
///
/// # Examples
///
/// Example of using a debounce:
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_timers::use_debounce;
/// use std::time::Duration;
///
/// #[component]
/// fn App() -> Element {
///     // Create a two second debounce.
///     // This will print "ran" after two seconds since the last action call.
///     let mut debounce = use_debounce(Duration::from_secs(2), |_| println!("ran"));
///
///     rsx! {
///         button {
///             onclick: move |_| {
///                 // Call the debounce.
///                 debounce.action(());
///             },
///             "Click!"
///         }
///     }
/// }
/// ```
///
/// #### Cancelling A Debounce
/// If you need to cancel the currently active debounce, you can call [`UseDebounce::cancel`]:
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_timers::use_debounce;
/// use std::time::Duration;
///
/// #[component]
/// fn App() -> Element {
///     let mut debounce = use_debounce(Duration::from_secs(5), |_| println!("ran"));
///
///     rsx! {
///         button {
///             // Start the debounce on click.
///             onclick: move |_| debounce.action(()),
///             "Action!"
///         }
///         button {
///             // Cancel the debounce on click.
///             onclick: move |_| debounce.cancel(),
///             "Cancel!"
///         }
///     }
/// }
/// ```
///
/// ### Async Debounce
/// Debounces can accept an async callback:
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_timers::use_debounce;
/// use std::time::Duration;
///
/// #[component]
/// fn App() -> Element {
///     // Create a two second debounce that uses some async/await.
///     let mut debounce = use_debounce(Duration::from_secs(2), |_| async {
///         println!("debounce called!");
///         tokio::time::sleep(Duration::from_secs(2)).await;
///         println!("after async");
///     });
///
///     rsx! {
///         button {
///             onclick: move |_| {
///                 // Call the debounce.
///                 debounce.action(());
///             },
///             "Click!"
///         }
///     }
/// }
/// ```
pub fn use_debounce<Args: 'static, MaybeAsync: SpawnIfAsync<Marker>, Marker>(
    duration: Duration,
    callback: impl FnMut(Args) -> MaybeAsync + 'static,
) -> UseDebounce<Args> {
    let timeout = use_timeout(duration, callback);
    let current_handle = use_signal(|| None);

    UseDebounce {
        timeout,
        current_handle,
    }
}
