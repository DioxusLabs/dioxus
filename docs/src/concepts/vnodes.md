# VNodes and Elements

At the heart of Dioxus is the concept of an "element" - a container that can have children, properties, event handlers, and other important attributes. Dioxus only knows how to render the `VNode` data structure - an Enum variant of an Element, Text, Components, Fragments, and Anchors.

Because Dioxus is meant for the Web and uses WebView as a desktop and mobile renderer, almost all elements in Dioxus share properties with their HTML counterpart. When we declare our elements, we'll do so using HTML semantics:

```rust
rsx!(
    div {
        "hello world"
    }
)
```

As you would expect, this snippet would generate a simple hello-world div. In fact, we can render these nodes directly with the SSR crate:

```rust
dioxus::ssr::render_lazy(rsx!(
    div {
        "hello world"
    }
))
```

And produce the corresponding html structure:
```html
<div>hello world</div>
```

Our structure declared above is made of two variants of the `VNode` data structure:
- A VElement with a tag name of `div`
- A VText with contents of `"hello world"`

## All the VNode types

VNodes can be any of:
- **Element**: a container with a tag name, namespace, attributes, children, and event listeners
- **Text**: bump allocated text derived from string formatting
- **Fragments**: a container of elements with no parent
- **Suspended**: a container for nodes that aren't yet ready to be rendered
- **Anchor**: a special type of node that is only available when fragments have no children

In practice, only elements and text can be initialized directly while other node types can only be created through hooks or NodeFactory methods.

## Bump Arena Allocation

To speed up the process of building our elements and text, Dioxus uses a special type of memory allocator tuned for large batches of small allocations called a Bump Arena. We use the `bumpalo` allocator which was initially developed for Dioxus' spiritual predecessor: `Dodrio.`

- Bumpalo: [https://github.com/fitzgen/bumpalo](https://github.com/fitzgen/bumpalo)
- Dodrio: [https://github.com/fitzgen/dodrio](https://github.com/fitzgen/dodrio)

In other frontend frameworks for Rust, nearly every string is allocated using the global allocator. This means that strings in Rust do not benefit from the immutable string interning optimizations that JavaScript engines employ. By using a smaller, faster, more limited allocator, we can increase framework performance, bypassing even the naive wasm-bindgen benchmarks for very quick renders.

It's important to note that VNodes are not `'static` - the VNode definition has a lifetime attached to it:

```rust, ignore
enum VNode<'bump> {
    VElement { tag: &'static str, children: &'bump [VNode<'bump>] },
    VText { content: &'bump str },
    // other VNodes ....
}
```

Because VNodes use a bump allocator as their memory backing, they can only be created through the `NodeFactory` API - which we'll cover in the next chapter. This particular detail is important to understand because "rendering" VNodes produces a lifetime attached to the bump arena - which must be explicitly declared when dealing with components that borrow data from their parents.
