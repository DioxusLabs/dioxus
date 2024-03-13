use dioxus_core::prelude::*;
use dioxus_signals::ReactiveContext;
use futures_util::StreamExt;

use crate::use_signal;
use dioxus_signals::*;

/// `use_effect` will subscribe to any changes in the signal values it captures
/// effects will always run after first mount and then whenever the signal values change
/// If the use_effect call was skipped due to an early return, the effect will no longer activate.
/// ```rust
/// # use dioxus::prelude::*;
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
pub fn use_effect(mut callback: impl FnMut() + 'static) -> Effect {
    // let mut run_effect = use_hook(|| CopyValue::new(true));
    // use_hook_did_run(move |did_run| run_effect.set(did_run));

    let location = std::panic::Location::caller();

    use_hook(|| {
        let (rc, mut changed) = ReactiveContext::new_with_origin(location);
        spawn(async move {
            loop {
                // Run the effect
                rc.run_in(&mut callback);

                // Wait for context to change
                let _ = changed.next().await;

                // Wait for the dom the be finished with sync work
                wait_for_next_render().await;
            }
        });
        Effect { rc }
    })
}

/// A handle to an effect.
#[derive(Clone, Copy)]
pub struct Effect {
    rc: ReactiveContext,
}

impl Effect {
    /// Adds an explicit dependency to the effect. If the dependency changes, the effect's closure will rerun.
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
    ///     // Because the effect subscribes to `delay` by adding it as a dependency, the effect's closure will rerun every time `delay` changes.
    ///     use_effect(move || {
    ///         println!("It is safe to manually manipulate the dom here");
    ///     })
    ///     .use_dependencies((&delay,));
    ///
    ///     todo!()
    /// }
    /// ```
    pub fn use_dependencies(mut self, dependency: impl Dependency) -> Self {
        let mut dependencies_signal = use_signal(|| dependency.out());
        let changed = { dependency.changed(&*dependencies_signal.read()) };
        if changed {
            dependencies_signal.set(dependency.out());
            self.mark_dirty();
        }
        self
    }

    /// Marks the effect as dirty, causing it to rerun on the next render.
    pub fn mark_dirty(&mut self) {
        self.rc.mark_dirty();
    }
}
