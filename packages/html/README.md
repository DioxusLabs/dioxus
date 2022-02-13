# Html (and SVG) Namespace for Dioxus

The Dioxus `rsx!` and `html!` macros can accept any compile-time correct namespace on top of NodeFactory. This crate provides the HTML (and SVG) namespaces which get imported in the Dioxus prelude.

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

## How it works:

Elements for dioxus must implement the (simple) DioxusElement trait to be used in the rsx! macro.

```rust
struct div;
impl DioxusElement for div {
    const TAG_NAME: &'static str = "div";
    const NAME_SPACE: Option<&'static str> = None;
}
```

All elements should be defined as a zero-sized-struct (also known as unit struct). These structs are zero-cost and just provide the type-level trickery to Rust for compile-time correct templates.

Attributes would then be implemented as methods on these unit structs.

The HTML namespace is defined mostly with macros. However, the expanded form would look something like this:
```rust
struct base;
impl DioxusElement for base {
    const TAG_NAME: &'static str = "base";
    const NAME_SPACE: Option<&'static str> = None;
}
impl base {
    #[inline]
    fn href<'a>(&self, f: NodeFactory<'a>, v: Arguments) -> Attribute<'a> {
        f.attr("href", v, None, false)
    }
    #[inline]
    fn target<'a>(&self, f: NodeFactory<'a>, v: Arguments) -> Attribute<'a> {
        f.attr("target", v, None, false)
    }
}
```
Because attributes are defined as methods on the unit struct, they guard the attribute creation behind a compile-time correct interface.


## How to extend it:

Whenever the rsx! macro is called, it relies on a module `dioxus_elements` to be in scope. When you enable the `html` feature in dioxus, this module gets imported in the prelude. However, you can extend this with your own set of custom elements by making your own `dioxus_elements` module and re-exporting the html namespace.

```rust
mod dioxus_elements {
    use dioxus::prelude::dioxus_elements::*;
    struct my_element;
    impl DioxusElement for my_element {
        const TAG_NAME: &'static str = "base";
        const NAME_SPACE: Option<&'static str> = None;
    }
}
```

## Limitations:
-

## How to work around it:
If an attribute in Dioxus is invalid (defined incorrectly) - first, make an issue - but then, you can work around it. The raw builder API is actually somewhat ergonomic to work with, and the NodeFactory type exposes a bunch of methods to make any type of tree - even invalid ones! So obviously, be careful, but there's basically anything you can do.

```rust
cx.render(rsx!{
    div {
        h1 {}
        // Oh no! I need a super custom element
        {LazyNodes::new(move |f| {
            f.raw_element(
                // tag name
                "custom_element",

                // attributes
                &[f.attr("billy", format_args!("goat"))],

                // listeners
                &[f.listener(onclick(move |_| {}))],

                // children
                &[cx.render(rsx!(div {} ))],

                // key
                None
            )
        })}
    }
})
```
