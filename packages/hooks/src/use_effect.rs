use dioxus_core::prelude::*;
use dioxus_signals::ReactiveContext;

/// `use_effect` will subscribe to any changes in the signal values it captures
/// effects will always run after first mount and then whenever the signal values change
/// If the use_effect call was skipped due to an early return, the effect will no longer activate.
/// ```rust
/// fn app() -> Element {
///     let mut count = use_signal(|| 0);
///     //the effect runs again each time count changes
///     use_effect(move || println!("Count changed to {count}"));
///
///     rsx! {
///         h1 { "High-Five counter: {count}" }
///         button { onclick: move |_| count += 1, "Up high!" }
///         button { onclick: move |_| count -= 1, "Down low!" }
///     }
/// }
/// ```
#[track_caller]
pub fn use_effect(mut callback: impl FnMut() + 'static) {
    // let mut run_effect = use_hook(|| CopyValue::new(true));
    // use_hook_did_run(move |did_run| run_effect.set(did_run));

    let location = std::panic::Location::caller();

    use_hook(|| {
        spawn(async move {
            let rc = ReactiveContext::new_with_origin(location);
            loop {
                // Wait for the dom the be finished with sync work
                // flush_sync().await;

                // Run the effect
                rc.run_in(&mut callback);

                // Wait for context to change
                rc.changed().await;
            }
        });
    });
}
