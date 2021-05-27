# Recoil.rs - Official global state management solution for Dioxus Apps

Recoil.rs provides a global state management API for Dioxus apps built on the concept of "atomic state." Instead of grouping state together into a single bundle ALA Redux, Recoil provides individual building blocks of state called Atoms. These atoms can be set/get anywhere in the app and combined to craft complex state. Recoil should be easier to learn and more efficient than Redux. Recoil.rs is modeled after the Recoil.JS project.

Recoil.rs is officially supported by the Dioxus team. By doing so, are are "planting our flag in the stand" for atomic state management instead of bundled (Redux-style) state management. Atomic state management fits well with the internals of Dioxus, meaning Recoil.rs state management will be faster, more efficient, and less sensitive to data races than Redux-style apps.

Internally, Dioxus uses batching to speed up linear-style operations. Recoil.rs integrates with this batching optimization, making app-wide changes extremely fast. This way, Recoil.rs can be pushed significantly harder than Redux without the need to enable/disable debug flags to prevent performance slowdowns.

## Guide

A simple atom of state is defined globally as a const:

```rust
const Light: Atom<&'static str> = |_| "Green";
```

This atom of state is initialized with a value of `"Green"`. The atom that is returned does not actually contain any values. Instead, the atom's key - which is automatically generated in this instance - is used in the context of a Recoil App.

This is then later used in components like so:

```rust
fn App(ctx: Context, props: &()) -> DomTree {
    // The recoil root must be initialized at the top of the application before any use_recoil hooks
    recoil::init_recoil_root(&ctx, |_| {});

    let color = recoil::use_read(ctx, Light);

    ctx.render(rsx!{
        h1 {"Color of light: {color}"}
    })
}
```
