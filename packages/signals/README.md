# Dioxus Signals

Dioxus Signals is an ergonomic Copy runtime for data with local subscriptions in Dioxus.

## Copy Data

All signals implement Copy, even if the inner value does not implement copy. This makes it easy to move any data into futures or children.

```rust
fn App(cx: Scope) -> Element {
    let signal = use_signal(cx, || "hello world".to_string());

    spawn(async move {
        // signal is Copy even though String is not copy
        signal
    });

    render! {
        "{signal}"
    }
}
```

## Local Subscriptions

Signals will only subscribe to components when you read from the signal in that component. It will never subscribe to a component when reading data in a future or event handler.

```rust
fn app(cx: Scope) -> Element {
    // Because signal is never read in this component, this component will not rerun when the signal changes
    let signal = use_signal(cx, || 0);

    render! {
        onclick: move |_| {
            *signal.write() += 1;
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

fn Child(cx: Scope<ChildProps>) -> Element {
    // This component does read from the signal, so when the signal changes it will rerun
    render! {
        "{cx.props.signal}"
    }
}
```
