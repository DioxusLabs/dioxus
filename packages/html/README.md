# Html (and SVG) Namespace for Dioxus

The Dioxus `rsx!` and `html!` macros can accept any compile-time correct namespace on top of NodeBuilder. This crate provides the HTML (and SVG) namespaces which get imported in the Dioxus prelude.

However, this abstraction enables you to add any namespace of elements, provided they're in scope when rsx! is called. For an example, a UI that is designed for Augmented Reality might use different primitives than HTML:

```rust
use ar_namespace::*;

rsx! {
    magic_div {
        magic_header {}
        magic_paragraph {
            on_magic_click: move |event| {
                //
            }
        }
    }
}
```

This is currently a not-very-explored part of Dioxus. However, the namespacing system does make it possible to provide syntax highlighting, documentation, "go to definition" and compile-time correctness, so it's worth having it abstracted.
