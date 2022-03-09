# Fermi

After having covered local and global state, you're definitely ready to start building some complex Dioxus apps. Before you get too far, check out the Fermi crate. Fermi makes it dead-easy to manage global state and scales to the largest of apps.

## What is Fermi for?

If you're building an app that has scaled past a few small components, then it'll be worth sketching out and organizing your app's state. Fermi makes it easy to transition from a simple app that relies on `use_state` to an app with dozens of components.

> To put it simply - Fermi is the ideal crate for managing global state in your Dioxus app.


## How do I use it?

To add fermi to your project, simply add the "fermi" feature to your Dioxus dependency.

```toml
[dependencies]
dioxus = { version = "0.2", features = ["desktop", "fermi"] }
```

Fermi is built on the concept of "atoms" of global state. Instead of bundling all our state together in a single mega struct, we actually chose to implement it piece-by-piece with functions.

Each piece of global state in your app is represented by an "atom."

```rust
static TITLE: Atom<String> = |_| "Defualt Title".to_string();
```

This atom can be then used the with the `use_atom` hook as a drop-in replacement for `use_state`.

```rust
static TITLE: Atom<String> = |_| "Defualt Title".to_string();

fn Title(cx: Scope) -> Element {
    let title = use_atom(&cx, TITLE);

    cx.render(rsx!{
        button { onclick: move |_| title.set("new title".to_string()) }
    })
}
```

However, Fermi really becomes useful when we want to share the value between two different components.

```rust
static TITLE: Atom<String> = |_| "Defualt Title".to_string();

fn TitleBar(cx: Scope) -> Element {
    let title = use_atom(&cx, TITLE);

    rsx!{cx, button { onclick: move |_| title.set("titlebar title".to_string()) } }
}

fn TitleCard(cx: Scope) -> Element {
    let title = use_atom(&cx, TITLE);

    rsx!{cx, button { onclick: move |_| title.set("title card".to_string()) } }
}
```

These two components can get and set the same value!


## Use with collections

Fermi gets *really* powerful when used to manage collections of data. Under the hood, Fermi uses immutable collections and tracks reads and writes of individual keys. This makes it easy to implement things like lists and infinite posts with little performance penalty. It also makes it really easy to refactor our app and add new fields.

```rust
static NAMES: AtomRef<Uuid, String> = |builder| {};
static CHECKED: AtomRef<Uuid, bool> = |builder| {};
static CREATED: AtomRef<Uuid, Instant> = |builder| {};
```

To use these collections:

```rust
#[inline_props]
fn Todo(cx: Scope, id: Uuid) -> Element {
    let name = use_atom(&cx, NAMES.select(id));
    let checked = use_atom(&cx, CHECKED.select(id));
    let created = use_atom(&cx, CREATED.select(id));

    // or

    let (name, checked, created) = use_atom(&cx, (NAMES, CHECKED, CREATED).select(id));
}
```

This particular pattern might seem strange at first - "why isn't all of our state under one struct?" - but eventually shows its worth when dealing with large amounts of data. By composing collections together, we can get get the perks of the Entity-Component-System architecture in our Dioxus apps. Performance is quite predictable here and easy to trace.

## AtomRef

Much like `use_ref` can be used to manage complex state locally, `AtomRef` can be used to manage complex global state. `AtomRef` is basically a global `Rc<RefCell<T>>` with mutation tracking.

It too serves as a basic replacement for `use_ref`:

```rust
fn Todo(cx: Scope) -> Element {
    let cfg = use_atom_ref(&cx, CFG);

    cfg.write().enable_option();
}

```

## Future Reading

This guide is meant just to be an overview of Fermi. This page is just a short advertisement for what we consider the best state management solution available today for Dioxus apps.

For further reading, check out the [crate itself](https://github.com/DioxusLabs/dioxus/tree/master/packages/fermi)!
