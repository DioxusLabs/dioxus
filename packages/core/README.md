# Dioxus-core

This is the core crate for the Dioxus Virtual DOM. This README will focus on the technical design and layout of this Virtual DOM implementation. If you want to read more about using Dioxus, then check out the Dioxus crate, documentation, and website.

To build new apps with Dioxus or to extend the ecosystem with new hooks or components, use the `Dioxus` crate with the appropriate feature flags.

## Internals

Dioxus-core builds off the many frameworks that came before it. Notably, Dioxus borrows these concepts:

- React: hooks, concurrency, suspense
- Dodrio: bump allocation, double buffering, and some diffing architecture
- Percy: html! macro architecture, platform-agnostic edits
- InfernoJS: approach to keyed diffing
- Preact: approach for normalization and ref
- Yew: passion and inspiration ❤️

Dioxus-core leverages some really cool techniques and hits a very high level of parity with mature frameworks. Some unique features include:

- managed lifetimes for borrowed data
- suspended nodes (task/fiber endpoints) for asynchronous vnodes
- custom memory allocator for vnodes and all text content
- support for fragments w/ lazy normalization
- slab allocator for scopes
- mirrored-slab approach for remote vdoms

There's certainly more to the story, but these optimizations make Dioxus memory use and allocation count extremely minimal. For an average application, it is likely that zero allocations will need to be performed once the app has been mounted. Only when new components are added to the dom will allocations occur - and only en mass. The space of old VNodes is dynamically recycled as new nodes are added. Additionally, Dioxus tracks the average memory footprint of previous components to estimate how much memory allocate for future components.

All in all, Dioxus treats memory as an incredibly valuable resource. Combined with the memory-efficient footprint of WASM compilation, Dioxus apps can scale to thousands of components and still stay snappy and respect your RAM usage.

## Goals

We have big goals for Dioxus. The final implementation must:

- Be **fast**. Allocators are typically slow in WASM/Rust, so we should have a smart way of allocating.
- Be extremely memory efficient. Servers should handle tens of thousands of simultaneous VDoms with no problem.
- Be concurrent. Components should be able to pause rendering using a threading mechanism.
- Be "remote". Edit lists should be separate from the Renderer implementation.
- Support SSR. VNodes should render to a string that can be served via a web server.
- Be "live". Components should be able to be both server rendered and client rendered without needing frontend APIs.
- Be modular. Components and hooks should be work anywhere without worrying about target platform.

## Optimizations

- Support a pluggable allocation strategy that makes VNode creation **very** fast
- Support lazy VNodes (ie VNodes that are not actually created when the html! macro is used)
- Support advanced diffing strategies (patience, Meyers, etc)

```rust

rsx!{ "this is a text node" }

rsx!{
    div {}
    "asd"
    div {}
    div {}
}
rsx!{
    div {
        a {}
        b {}
        c {}
        Container {
            Container {
                Container {
                    Container {
                        Container {
                            div {}
                        }
                    }
                }
            }
        }
    }
}




```
