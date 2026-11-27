use dioxus_signals::{Signal, WritableExt as _};
use dioxus_vdom::{Callback, SpawnIfAsync, Task, spawn, use_hook};
use std::time::Duration;

/// The interface to a debounce.
///
/// You can cancel an interval with [`UseInterval::cancel`].
/// See [`use_interval`] for more information.
#[derive(Clone, PartialEq, Copy)]
pub struct UseInterval {
    inner: Signal<InnerUseInterval>,
}

struct InnerUseInterval {
    pub(crate) interval: Option<Task>,
}

impl Drop for InnerUseInterval {
    fn drop(&mut self) {
        if let Some(interval) = self.interval.take() {
            interval.cancel();
        }
    }
}

impl UseInterval {
    /// Cancel the interval.
    pub fn cancel(&mut self) {
        if let Some(interval) = self.inner.write().interval.take() {
            interval.cancel();
        }
    }
}

/// Repeatedly call a function at a specific interval.
///
/// Intervals are cancelable with the [`UseInterval::cancel`] method.
///
/// # Examples
///
/// Example of using an interval:
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_timers::use_interval;
/// use std::time::Duration;
///
/// #[component]
/// fn App() -> Element {
///     let mut time_elapsed = use_signal(|| 0);
///     // Create an interval that increases the time elapsed signal by one every second.
///     use_interval(Duration::from_secs(1), move |()| time_elapsed += 1);
///
///     rsx! {
///         "It has been {time_elapsed} since the app started."
///     }
/// }
/// ```
///
/// #### Cancelling Intervals
/// Example of cancelling an interval:
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_timers::use_interval;
/// use std::time::Duration;
///
/// #[component]
/// fn App() -> Element {
///     let mut time_elapsed = use_signal(|| 0);
///     let mut interval = use_interval(Duration::from_secs(1), move |()| time_elapsed += 1);
///
///     rsx! {
///         "It has been {time_elapsed} since the app started."
///         button {
///             // Cancel the interval when the button is clicked.
///             onclick: move |_| interval.cancel(),
///             "Cancel Interval"
///         }
///     }
/// }
/// ```
///
/// #### Async Intervals
/// Intervals can accept an async callback:
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_timers::use_interval;
/// use std::time::Duration;
///
/// #[component]
/// fn App() -> Element {
///     let mut time_elapsed = use_signal(|| 0);
///     // Create an interval that increases the time elapsed signal by one every second.
///     use_interval(Duration::from_secs(1), move |()| async move {
///         time_elapsed += 1;
///         // Pretend we're doing some async work.
///         tokio::time::sleep(Duration::from_secs(1)).await;
///         println!("Done!");
///     });
///
///     rsx! {
///         "It has been {time_elapsed} since the app started."
///     }
/// }
/// ```
pub fn use_interval<MaybeAsync: SpawnIfAsync<Marker>, Marker>(
    period: Duration,
    callback: impl FnMut(()) -> MaybeAsync + 'static,
) -> UseInterval {
    let inner = use_hook(|| {
        let callback = Callback::new(callback);

        let task = spawn(async move {
            #[cfg(not(target_family = "wasm"))]
            let mut interval = tokio::time::interval(period);

            loop {
                #[cfg(not(target_family = "wasm"))]
                interval.tick().await;

                #[cfg(target_family = "wasm")]
                {
                    gloo_timers::future::sleep(period).await;
                }

                callback.call(());
            }
        });

        Signal::new(InnerUseInterval {
            interval: Some(task),
        })
    });

    UseInterval { inner }
}
