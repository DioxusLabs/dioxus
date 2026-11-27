use dioxus_signals::Signal;
use dioxus_vdom::{Callback, SpawnIfAsync, Task, spawn, use_hook};
use futures::{SinkExt, StreamExt, channel::mpsc};
use std::time::Duration;

/// The interface to a timeout.
///
/// This is used to trigger the timeout with [`UseTimeout::action`].
///
/// See [`use_timeout`] for more information.
pub struct UseTimeout<Args: 'static> {
    duration: Duration,
    sender: Signal<mpsc::UnboundedSender<Args>>,
}

impl<Args> UseTimeout<Args> {
    /// Trigger the timeout.
    ///
    /// If no arguments are desired, use the [`unit`] type.
    /// See [`use_timeout`] for more information.
    pub fn action(&self, args: Args) -> TimeoutHandle {
        let mut sender = (self.sender)();
        let duration = self.duration;

        let handle = spawn(async move {
            #[cfg(not(target_family = "wasm"))]
            tokio::time::sleep(duration).await;

            #[cfg(target_family = "wasm")]
            gloo_timers::future::sleep(duration).await;

            // If this errors then the timeout was likely dropped.
            let _ = sender.send(args).await;
        });

        TimeoutHandle { handle }
    }
}

impl<Args> Clone for UseTimeout<Args> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<Args> Copy for UseTimeout<Args> {}
impl<Args> PartialEq for UseTimeout<Args> {
    fn eq(&self, other: &Self) -> bool {
        self.duration == other.duration && self.sender == other.sender
    }
}

/// A handle to a pending timeout.
///
/// A handle to a running timeout triggered with [`UseTimeout::action`].
/// This handle allows you to cancel the timeout from triggering with [`TimeoutHandle::cancel`]
///
/// See [`use_timeout`] for more information.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeoutHandle {
    handle: Task,
}

impl TimeoutHandle {
    /// Cancel the timeout associated with this handle.
    pub fn cancel(self) {
        self.handle.cancel();
    }
}

/// A hook to run a callback after a period of time.
///
/// Timeouts allow you to trigger a callback that occurs after a period of time. Unlike a debounce, a timeout will not
/// reset it's timer when triggered again. Instead, calling a timeout while it is already running will start another instance
/// to run the callback after the provided period.
///
/// This hook is similar to the web [setTimeout()](https://developer.mozilla.org/en-US/docs/Web/API/Window/setTimeout) API.
///
/// # Examples
///
/// Example of using a timeout:
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_timers::use_timeout;
/// use std::time::Duration;
///
/// #[component]
/// fn App() -> Element {
///     // Create a timeout for two seconds.
///     // Once triggered, this timeout will print "timeout called" after two seconds.
///     let timeout = use_timeout(Duration::from_secs(2), |()| println!("timeout called"));
///
///     rsx! {
///         button {
///             onclick: move |_| {
///                 // Trigger the timeout.
///                 timeout.action(());
///             },
///             "Click!"
///         }
///     }
/// }
/// ```
///
/// #### Cancelling Timeouts
/// Example of cancelling a timeout. This is the equivalent of a debounce.
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_timers::{use_timeout, TimeoutHandle};
/// use std::time::Duration;
///
/// #[component]
/// fn App() -> Element {
///     let mut current_timeout: Signal<Option<TimeoutHandle>> = use_signal(|| None);
///     let timeout = use_timeout(Duration::from_secs(2), move |()| {
///         current_timeout.set(None);
///         println!("timeout called");
///     });
///
///     rsx! {
///         button {
///             onclick: move |_| {
///                 // Cancel any currently running timeouts.
///                 if let Some(handle) = *current_timeout.read() {
///                     handle.cancel();
///                 }
///
///                 // Trigger the timeout.
///                 let handle = timeout.action(());
///                 current_timeout.set(Some(handle));
///             },
///             "Click!"
///         }
///     }
/// }
/// ```
///
/// #### Async Timeouts
/// Timeouts can accept an async callback:
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_timers::use_timeout;
/// use std::time::Duration;
///
/// #[component]
/// fn App() -> Element {
///     // Create a timeout for two seconds.
///     // We use an async sleep to wait an even longer duration after the timeout is called.
///     let timeout = use_timeout(Duration::from_secs(2), |()| async {
///         println!("Timeout after two total seconds.");
///         tokio::time::sleep(Duration::from_secs(2)).await;
///         println!("Timeout after four total seconds.");
///     });
///
///     rsx! {
///         button {
///             onclick: move |_| {
///                 // Trigger the timeout.
///                 timeout.action(());
///             },
///             "Click!"
///         }
///     }
/// }
/// ```
pub fn use_timeout<Args: 'static, MaybeAsync: SpawnIfAsync<Marker>, Marker>(
    duration: Duration,
    callback: impl FnMut(Args) -> MaybeAsync + 'static,
) -> UseTimeout<Args> {
    use_hook(|| {
        let callback = Callback::new(callback);
        let (sender, mut receiver) = mpsc::unbounded();

        spawn(async move {
            loop {
                if let Some(args) = receiver.next().await {
                    callback.call(args);
                }
            }
        });

        UseTimeout {
            duration,
            sender: Signal::new(sender),
        }
    })
}
