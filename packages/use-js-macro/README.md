# Dioxus call-js

A macro to simplify calling javascript from rust

## Usage
Add `dioxus-call-js` to your `Cargo.toml`:
```toml
[dependencies]
dioxus-call-js = "0.1"
```

Example:
```rust
use dioxus::prelude::*;
use dioxus_call_js::call_js;

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    let future = use_resource(|| async move {
        let from = "dave";
        let to = "john";
        let greeting = call_js!("assets/example.js", greeting(from, to)).await.unwrap();
        let greeting: String = serde_json::from_value(greeting).unwrap();
        return greeting;
    });

    rsx!(
        div {
            h1 { "Dioxus `call_js!` macro example!" }
            {
                match &*future.read() {
                    Some(greeting) => rsx! {
                        p { "Greeting from JavaScript: {greeting}" }
                    },
                    None => rsx! {
                        p { "Running js" }
                    },
                }
            }
        }
    )
}
```