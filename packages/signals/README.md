# Dioxus Signals

Dioxus Signals is an ergonomic Copy runtime for data with local subscriptions.

## Copy Data

All signals implement Copy, even if the inner value does not implement copy. This makes it easy to move any data into futures or children.

```rust
use dioxus::prelude::*;
use dioxus_signals::*;

#[component]
fn App() -> Element {
    let signal = use_signal(|| "hello world".to_string());

    spawn(async move {
        // signal is Copy even though String is not copy
        print!("{signal}");
    });

    rsx! {
        "{signal}"
    }
}
```

## Local Subscriptions

Signals will only subscribe to components when you read from the signal in that component. It will never subscribe to a component when reading data in a future or event handler.

```rust
use dioxus::prelude::*;
use dioxus_signals::*;

#[component]
fn App() -> Element {
    // Because signal is never read in this component, this component will not rerun when the signal changes
    let signal = use_signal(|| 0);

    rsx! {
        button {
            onclick: move |_| {
                *signal.write() += 1;
            },
            "Increase"
        }
        for id in 0..10 {
            Child {
                signal: signal,
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ChildProps {
    signal: Signal<usize>,
}

#[component]
fn Child(cx: Scope<ChildProps>) -> Element {
    // This component does read from the signal, so when the signal changes it will rerun
    rsx! {
        "{cx.props.signal}"
    }
}
```

Because subscriptions happen when you read from (not create) the data, you can provide signals through the normal context API:

```rust
use dioxus::prelude::*;
use dioxus_signals::*;

#[component]
fn App() -> Element {
    // Because signal is never read in this component, this component will not rerun when the signal changes
    use_context_provider(|| Signal::new(0));

    rsx! {
        Child {}
    }
}

#[component]
fn Child() -> Element {
    let signal: Signal<i32> = *use_context(cx).unwrap();
    // This component does read from the signal, so when the signal changes it will rerun
    rsx! {
        "{signal}"
    }
}
```

## Computed Data

In addition to local subscriptions in components, `dioxus-signals` provides a way to derive data with local subscriptions.

The use_memo hook will only rerun when any signals inside the hook change:

```rust
use dioxus::prelude::*;
use dioxus_signals::*;

#[component]
fn App() -> Element {
    let signal = use_signal(|| 0);
    let doubled = use_memo(|| signal * 2);

    rsx! {
        button {
            onclick: move |_| *signal.write() += 1,
            "Increase"
        }
        Child {
            signal: doubled
        }
    }
}

#[component]
fn Child(signal: ReadOnlySignal<usize>) -> Element {
    rsx! {
        "{signal}"
    }
}
```
