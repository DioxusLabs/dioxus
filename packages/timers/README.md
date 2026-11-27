# dioxus-timers

Cross-platform timers and timer hooks for Dioxus apps:

- `use_interval`: run a callback on a fixed interval
- `use_timeout`: run a callback once after a delay
- `use_debounce`: run a callback after a delay, restarting the delay on every call
- `sleep`: an async sleep that works on every platform

On wasm targets these use browser timers (`gloo-timers`); everywhere else they use `tokio::time`, so they must be polled inside a tokio runtime.

## Usage

Enable the `timers` feature of `dioxus` and use `dioxus::timers`, or depend on `dioxus-timers` directly.

```rust, no_run
use dioxus::prelude::*;
use dioxus::timers::{use_debounce, use_interval};
use std::time::Duration;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut count = use_signal(|| 0);

    // Increment count every second.
    use_interval(Duration::from_secs(1), move |()| count += 1);

    // Reset count 2 seconds after the latest call.
    let mut debounce = use_debounce(Duration::from_millis(2000), move |()| count.set(0));

    rsx! {
        p { "{count}" }
        button { onclick: move |_| debounce.action(()), "Reset the counter (2 second debounce)" }
    }
}
```
