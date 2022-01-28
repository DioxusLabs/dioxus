# Hooks and Internal State

In the [Adding Interactivity](./interactivity.md) section, we briefly covered the concept of hooks and state stored internal to components.

In this section, we'll dive a bit deeper into hooks, exploring both the theory and mechanics.



## Theory of Hooks

Over the past several decades, computer scientists and engineers have long sought the "right way" of designing user interfaces. With each new programming language, novel features are unlocked that change the paradigm in which user interfaces are coded.

Generally, a number of patterns have emerged, each with their own strengths and tradeoffs.

Broadly, there are two types of GUI structures:

- Immediate GUIs: re-render the entire screen on every update
- Retained GUIs: only re-render the portion of the screen that changed

Typically, immediate-mode GUIs are simpler to write but can slow down as more features, like styling, are added.

Many GUIs today are written in *Retained mode* - your code changes the data of the user interface but the renderer is responsible for actually drawing to the screen. In these cases, our GUI's state sticks around as the UI is rendered. To help accommodate retained mode GUIs, like the web browser, Dioxus provides a mechanism to keep state around.

> Note: Even though hooks are accessible, you should still prefer one-way data flow and encapsulation. Your UI code should be as predictable as possible. Dioxus is plenty fast, even for the largest apps.

## Mechanics of Hooks
In order to have state stick around between renders, Dioxus provides the `hook` through the `use_hook` API. This gives us a mutable reference to data returned from the initialization function.

```rust
fn example(cx: Scope) -> Element {
    let name: &mut String = cx.use_hook(|| "John Doe".to_string());

    //
}
```

We can even modify this value directly from an event handler:

```rust
fn example(cx: Scope) -> Element {
    let name: &mut String = cx.use_hook(|| "John Doe".to_string());

    cx.render(rsx!(
        button {
            onclick: move |_| name.push_str(".."),
        }
    ))
}
```

Mechanically, each call to `use_hook` provides us with `&mut T` for a new value.

```rust
fn example(cx: Scope) -> Element {
    let name: &mut String = cx.use_hook(|| "John Doe".to_string());
    let age: &mut u32 = cx.use_hook(|| 10);
    let friends: &mut Vec<String> = cx.use_hook(|| vec!["Jane Doe".to_string()]);

    //
}
```

Internally, Dioxus is creating a list of hook values with each call to `use_hook` advancing the index of the list to return the next value.

Our internal HookList would look something like:

```rust
[
    Hook<String>,
    Hook<u32>,
    Hook<String>,
]
```

This is why hooks called out of order will fail - if we try to downcast a `Hook<String>` to `Hook<u32>`, Dioxus has no choice but to panic. We do provide a `try_use_hook` but you should never need that in practice.

This pattern might seem strange at first, but it can be a significant upgrade over structs as blobs of state, which tend to be difficult to use in [Rust given the ownership system](https://rust-lang.github.io/rfcs/2229-capture-disjoint-fields.html).


## Rules of hooks

Hooks are sensitive to how they are used. To use hooks, you must abide by the
"rules of hooks" (borrowed from react)](https://reactjs.org/docs/hooks-rules.html):

- Functions with "use_" should not be called in callbacks
- Functions with "use_" should not be called out of order
- Functions with "use_" should not be called in loops or conditionals

Examples of "no-nos" include:

### ❌ Nested uses

```rust
// ❌ don't call use_hook or any `use_` function *inside* use_hook!
cx.use_hook(|_| {
    let name = cx.use_hook(|_| "ads");
})

// ✅ instead, move the first hook above
let name = cx.use_hook(|_| "ads");
cx.use_hook(|_| {
    // do something with name here
})
```

### ❌ Uses in conditionals
```rust
// ❌ don't call use_ in conditionals!
if do_thing {
    let name = use_state(&cx, || 0);
}

// ✅ instead, *always* call use_state but leave your logic
let name = use_state(&cx, || 0);
if do_thing {
    // do thing with name here
}
```

### ❌ Uses in loops


```rust
// ❌ Do not use hooks in loops!
let mut nodes = vec![];

for name in names {
    let age = use_state(&cx, |_| 0);
    nodes.push(cx.render(rsx!{
        div { "{age}" }
    }))
}

// ✅ Instead, consider refactoring your usecase into components 
#[inline_props]
fn Child(cx: Scope, name: String) -> Element {
    let age = use_state(&cx, |_| 0);
    cx.render(rsx!{ div { "{age}" } })
}

// ✅ Or, use a hashmap with use_ref
```rust
let ages = use_ref(&cx, |_| HashMap::new());

names.iter().map(|name| {
    let age = ages.get(name).unwrap();
    cx.render(rsx!{ div { "{age}" } })
})
```

## Building new Hooks

However, most hooks you'll interact with *don't* return an `&mut T` since this is not very useful in a real-world situation.

Consider when we try to pass our `&mut String` into two different handlers:

```rust
fn example(cx: Scope) -> Element {
    let name: &mut String = cx.use_hook(|| "John Doe".to_string());

    cx.render(rsx!(
        button { onclick: move |_| name.push_str("yes"), }
        button { onclick: move |_| name.push_str("no"), }
    ))
}
```

Rust will not allow this to compile! We cannot `Copy` unique mutable references - they are, by definition, unique. However, we *can* reborrow our `&mut T` as an `&T` which are non-unique references and share those between handlers:

```rust
fn example(cx: Scope) -> Element {
    let name: &String = &*cx.use_hook(|| "John Doe".to_string());

    cx.render(rsx!(
        button { onclick: move |_| log::info!("{}", name), }
        button { onclick: move |_| log::info!("{}", name), }
    ))
}
```

So, for any custom hook we want to design, we need to enable mutation through [interior mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html) - IE move to runtime [borrow checking](https://doc.rust-lang.org/1.8.0/book/references-and-borrowing.html). We might incur a tiny runtime cost for each time we grab a new value from the hook, but this cost is extremely minimal.

This example uses the `Cell` type to let us replace the value through interior mutability. `Cell` has practically zero overhead, but is slightly more limited that its `RefCell` cousin.

```rust
fn example(cx: Scope) -> Element {
    let name: &Cell<&'static str> = cx.use_hook(|| Cell::new("John Doe"));

    cx.render(rsx!(
        button { onclick: move |_| name.set("John"), }
        button { onclick: move |_| name.set("Jane"), }
    ))
}
```

## Driving state updates through hooks

Hooks like `use_state` and `use_ref` wrap this runtime borrow checking in a type that *does* implement `Copy`. Additionally, they also mark the component as "dirty" whenever a new value has been set. This way, whenever `use_state` has a new value `set`, the component knows to update.

```rust
fn example(cx: Scope) -> Element {
    let name = use_state(&cx, || "Jack");

    cx.render(rsx!(
        "Hello, {name}"
        button { onclick: move |_| name.set("John"), }
        button { onclick: move |_| name.set("Jane"), }
    ))
}
```

Internally, our `set` function looks something like this:

```rust
impl<'a, T> UseState<'a, T> {
    fn set(&self, new: T) {
        // Replace the value in the cell
        self.value.set(new);

        // Mark our component as dirty
        self.cx.needs_update();
    }
}
```

Most hooks we provide implement `Deref` on their values since they are essentially smart pointers. To access the underlying value, you'll often need to use the deref operator:

```rust
fn example(cx: Scope) -> Element {
    let name = use_state(&cx, || "Jack");

    match *name {
        "Jack" => {}
        "Jill" => {}
        _ => {}
    }

    // ..
}

```


## Hooks provided by the `Dioxus-Hooks` package

By default, we bundle a handful of hooks in the Dioxus-Hooks package. Feel free to click on each hook to view its definition and associated documentation.

- [use_state](https://docs.rs/dioxus_hooks/use_state) - store state with ergonomic updates
- [use_ref](https://docs.rs/dioxus_hooks/use_ref) - store non-clone state with a refcell
- [use_future](https://docs.rs/dioxus_hooks/use_future) - store a future to be polled after initialization
- [use_coroutine](https://docs.rs/dioxus_hooks/use_coroutine) - store a future that can be stopped/started/communicated with
- [use_noderef](https://docs.rs/dioxus_hooks/use_noderef) - store a handle to the native element
- [use_callback](https://docs.rs/dioxus_hooks/use_callback) - store a callback that implements PartialEq for memoization
- [use_provide_context](https://docs.rs/dioxus_hooks/use_provide_context) - expose state to descendent components
- [use_context](https://docs.rs/dioxus_hooks/use_context) - consume state provided by `use_provide_context`

For a more in-depth guide to building new hooks, checkout out the advanced hook building guide in the reference.

## Wrapping up

In this chapter, we learned about the mechanics and intricacies of storing state inside a component.

In the next chapter, we'll cover event listeners in similar depth, and how to combine the two to build interactive components.
