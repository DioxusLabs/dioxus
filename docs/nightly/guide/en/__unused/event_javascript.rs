
## JavaScript Handlers

Instead of passing a closure, you can also pass a string to event handlers – this lets you use JavaScript (if your renderer can execute JavaScript):

```rust
{{#include ../../../examples/event_javascript.rs:rsx}}
```


#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        // ANCHOR: rsx
        div {
            onclick: "alert('hello world')",
        }
        // ANCHOR_END: rsx
    })
}
