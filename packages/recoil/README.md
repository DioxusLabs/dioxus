# Recoil.rs
Recoil.rs provides a global state management API for Dioxus apps built on the concept of "atomic state." Instead of grouping state together into a single bundle ALA Redux, Recoil provides individual building blocks of state called Atoms. These atoms can be set/get anywhere in the app and combined to craft complex state. Recoil should be easier to learn and more efficient than Redux. Recoil.rs is modeled after the Recoil.JS project and pulls in 


## Guide

A simple atom of state is defined globally as a const:

```rust
static Light: Atom<&'static str> = atom(|_| "Green");
```

This atom of state is initialized with a value of `"Green"`. The atom that is returned does not actually contain any values. Instead, the atom's key - which is automatically generated in this instance - is used in the context of a Recoil App.  

This is then later used in components like so:

```rust
fn App(ctx: Context, props: &()) -> DomTree {
    // The recoil root must be initialized at the top of the application before any uses 
    recoil::init_recoil_root(&ctx, |_| {});

    let color = use_recoil(&ctx, &TITLE);

    ctx.render(rsx!{
        h1 {"Color of light: {color}"}
    })
}
```
