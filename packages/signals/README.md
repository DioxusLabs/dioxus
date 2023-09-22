# Dioxus Signals

Dioxus Signals is an ergonomic Copy runtime for data with local subscriptions.

## Copy Data

All signals implement Copy, even if the inner value does not implement copy. This makes it easy to move any data into futures or children.

```rust
use dioxus::prelude::*;
use dioxus_signals::*;

#[component]
fn App(cx: Scope) -> Element {
    let signal = use_signal(cx, || "hello world".to_string());

    spawn(async move {
        // signal is Copy even though String is not copy
        print!("{signal}");
    });

    render! {
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
fn App(cx: Scope) -> Element {
    // Because signal is never read in this component, this component will not rerun when the signal changes
    let signal = use_signal(cx, || 0);

    render! {
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
    render! {
        "{cx.props.signal}"
    }
}
```

Because subscriptions happen when you read from (not create) the data, you can provide signals through the normal context API:

```rust
use dioxus::prelude::*;
use dioxus_signals::*;

#[component]
fn App(cx: Scope) -> Element {
    // Because signal is never read in this component, this component will not rerun when the signal changes
    use_context_provider(cx, || Signal::new(0));
    
    render! {
        Child {}
    }
}

#[component]
fn Child(cx: Scope) -> Element {
    let signal: Signal<i32> = *use_context(cx).unwrap();
    // This component does read from the signal, so when the signal changes it will rerun
    render! {
        "{signal}"
    }
}
```

## Computed Data

In addition to local subscriptions in components, `dioxus-signals` provides a way to derive data with local subscriptions.

The use_selector hook will only rerun when any signals inside the hook change:

```rust
use dioxus::prelude::*;
use dioxus_signals::*;

#[component]
fn App(cx: Scope) -> Element {
    let signal = use_signal(cx, || 0);
    let doubled = use_selector(cx, || signal * 2);

    render! {
        button {
            onclick: move |_| *signal.write() += 1,
            "Increase"
        }
        Child {
            signal: signal
        }
    }
}

#[component]
fn Child(cx: Scope, signal: ReadOnlySignal<usize>) -> Element {
    render! {
        "{signal}"
    }
}
```
