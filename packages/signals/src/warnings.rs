//! Warnings that can be triggered by suspicious usage of signals

use warnings::warning;

/// A warning that is triggered when a copy value is used in a higher scope that it is owned by
#[warning]
pub fn copy_value_hoisted<T: 'static, S: generational_box::Storage<T>>(
    value: &crate::CopyValue<T, S>,
    caller: &'static std::panic::Location<'static>,
) {
    let origin_scope = value.origin_scope;
    if let Ok(current_scope) = dioxus_core::prelude::current_scope_id() {
        // If the current scope is a descendant of the origin scope or is the same scope, we don't need to warn
        if origin_scope == current_scope || current_scope.is_descendant_of(origin_scope) {
            return;
        }
        let create_location = value.value.created_at().unwrap();
        let broken_example = include_str!("../docs/hoist/error.rs");
        let fixed_example = include_str!("../docs/hoist/fixed_list.rs");

        // Otherwise, warn that the value is being used in a higher scope and may be dropped
        tracing::warn!(
            r#"A Copy Value created in {origin_scope:?} (at {create_location}). It will be dropped when that scope is dropped, but it was used in {current_scope:?} (at {caller}) which is not a descendant of the owning scope.
This may cause reads or writes to fail because the value is dropped while it still held.

Help:
Copy values (like CopyValue, Signal, Memo, and Resource) are owned by the scope they are created in. If you use the value in a scope that may be dropped after the origin scope,
it is very easy to use the value after it has been dropped. To fix this, you can move the value to the parent of all of the scopes that it is used in.

Broken example ❌:
```rust
{broken_example}
```

Fixed example ✅:
```rust
{fixed_example}
```"#
        );
    }
}

// Include the examples from the warning to make sure they compile
#[test]
#[allow(unused)]
fn hoist() {
    mod broken {
        use dioxus::prelude::*;
        include!("../docs/hoist/error.rs");
    }
    mod fixed {
        use dioxus::prelude::*;
        include!("../docs/hoist/fixed_list.rs");
    }
}

/// Check if the write happened during a render. If it did, warn the user that this is generally a bad practice.
#[warning]
pub fn signal_write_in_component_body(origin: &'static std::panic::Location<'static>) {
    // Check if the write happened during a render. If it did, we should warn the user that this is generally a bad practice.
    if dioxus_core::vdom_is_rendering() {
        tracing::warn!(
            "Write on signal at {} happened while a component was running. Writing to signals during a render can cause infinite rerenders when you read the same signal in the component. Consider writing to the signal in an effect, future, or event handler if possible.",
            origin
        );
    }
}

/// Check if the write happened during a scope that the signal is also subscribed to. If it did, trigger a warning because it will likely cause an infinite loop.
#[warning]
pub fn signal_read_and_write_in_reactive_scope<
    T: 'static,
    S: generational_box::Storage<crate::SignalData<T>>,
>(
    origin: &'static std::panic::Location<'static>,
    signal: crate::Signal<T, S>,
) {
    // Check if the write happened during a scope that the signal is also subscribed to. If it did, this will probably cause an infinite loop.
    if let Some(reactive_context) = dioxus_core::prelude::ReactiveContext::current() {
        if let Ok(inner) = crate::Readable::try_read(&signal.inner) {
            if let Ok(subscribers) = inner.subscribers.lock() {
                for subscriber in subscribers.iter() {
                    if reactive_context == *subscriber {
                        tracing::warn!(
                            "Write on signal at {origin} finished in {reactive_context} which is also subscribed to the signal. This will likely cause an infinite loop. When the write finishes, {reactive_context} will rerun which may cause the write to be rerun again.\nHINT:\n{SIGNAL_READ_WRITE_SAME_SCOPE_HELP}",
                        );
                    }
                }
            }
        }
    }
}

#[allow(unused)]
const SIGNAL_READ_WRITE_SAME_SCOPE_HELP: &str = r#"This issue is caused by reading and writing to the same signal in a reactive scope. Components, effects, memos, and resources each have their own a reactive scopes. Reactive scopes rerun when any signal you read inside of them are changed. If you read and write to the same signal in the same scope, the write will cause the scope to rerun and trigger the write again. This can cause an infinite loop.

You can fix the issue by either:
1) Splitting up your state and Writing, reading to different signals:

For example, you could change this broken code:

#[derive(Clone, Copy)]
struct Counts {
    count1: i32,
    count2: i32,
}

fn app() -> Element {
    let mut counts = use_signal(|| Counts { count1: 0, count2: 0 });

    use_effect(move || {
        // This effect both reads and writes to counts
        counts.write().count1 = counts().count2;
    })
}

Into this working code:

fn app() -> Element {
    let mut count1 = use_signal(|| 0);
    let mut count2 = use_signal(|| 0);

    use_effect(move || {
        count1.set(count2());
    });
}
2) Reading and Writing to the same signal in different scopes:

For example, you could change this broken code:

fn app() -> Element {
    let mut count = use_signal(|| 0);

    use_effect(move || {
        // This effect both reads and writes to count
        println!("{}", count());
        count.set(1);
    });
}


To this working code:

fn app() -> Element {
    let mut count = use_signal(|| 0);

    use_effect(move || {
        count.set(1);
    });
    use_effect(move || {
        println!("{}", count());
    });
}
"#;
