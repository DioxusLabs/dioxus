# Error handling

A selling point of Rust for web development is the reliability of always knowing where errors can occur and being forced to handle them

However, we haven't talked about error handling at all in this guide! In this chapter, we'll cover some strategies in handling errors to ensure your app never crashes.



## The simplest – returning None

Astute observers might have noticed that `Element` is actually a type alias for `Option<VNode>`. You don't need to know what a `VNode` is, but it's important to recognize that we could actually return nothing at all:

```rust
fn App(cx: Scope) -> Element {
    None
}
```

This lets us add in some syntactic sugar for operations we think *shouldn't* fail, but we're still not confident enough to "unwrap" on.

> The nature of `Option<VNode>` might change in the future as the `try` trait gets upgraded.

```rust
fn App(cx: Scope) -> Element {
    // immediately return "None"
    let name = cx.use_hook(|_| Some("hi"))?;
}
```

## Early return on result

Because Rust can't accept both Options and Results with the existing try infrastructure, you'll need to manually handle Results. This can be done by converting them into Options or by explicitly handling them.

```rust
fn App(cx: Scope) -> Element {
    // Convert Result to Option
    let name = cx.use_hook(|_| "1.234").parse().ok()?;


    // Early return
    let count = cx.use_hook(|_| "1.234");
    let val = match count.parse() {
        Ok(val) => val
        Err(err) => return cx.render(rsx!{ "Parsing failed" })
    };
}
```

Notice that while hooks in Dioxus do not like being called in conditionals or loops, they *are* okay with early returns. Returning an error state early is a completely valid way of handling errors.


## Match results

The next "best" way of handling errors in Dioxus is to match on the error locally. This is the most robust way of handling errors, though it doesn't scale to architectures beyond a single component.

To do this, we simply have an error state built into our component:

```rust
let err = use_state(cx, || None);
```

Whenever we perform an action that generates an error, we'll set that error state. We can then match on the error in a number of ways (early return, return Element, etc).


```rust
fn Commandline(cx: Scope) -> Element {
    let error = use_state(cx, || None);

    cx.render(match *error {
        Some(error) => rsx!(
            h1 { "An error occured" }
        )
        None => rsx!(
            input {
                oninput: move |_| error.set(Some("bad thing happened!")),
            }
        )
    })
}
```

## Passing error states through components

If you're dealing with a handful of components with minimal nesting, you can just pass the error handle into child components.

```rust
fn Commandline(cx: Scope) -> Element {
    let error = use_state(cx, || None);

    if let Some(error) = **error {
        return cx.render(rsx!{ "An error occured" });
    }

    cx.render(rsx!{
        Child { error: error.clone() }
        Child { error: error.clone() }
        Child { error: error.clone() }
        Child { error: error.clone() }
    })
}
```

Much like before, our child components can manually set the error during their own actions. The advantage to this pattern is that we can easily isolate error states to a few components at a time, making our app more predictable and robust.

## Going global

A strategy for handling cascaded errors in larger apps is through signaling an error using global state. This particular pattern involves creating an "error" context, and then setting it wherever relevant. This particular method is not as "sophisticated" as React's error boundary, but it is more fitting for Rust.

To get started, consider using a built-in hook like `use_context` and `use_context_provider` or Fermi. Of course, it's pretty easy to roll your own hook too.

At the "top" of our architecture, we're going to want to explicitly declare a value that could be an error.


```rust
enum InputError {
    None,
    TooLong,
    TooShort,
}

static INPUT_ERROR: Atom<InputError> = |_| InputError::None;
```

Then, in our top level component, we want to explicitly handle the possible error state for this part of the tree.

```rust
fn TopLevel(cx: Scope) -> Element {
    let error = use_read(cx, INPUT_ERROR);

    match error {
        TooLong => return cx.render(rsx!{ "FAILED: Too long!" }),
        TooShort => return cx.render(rsx!{ "FAILED: Too Short!" }),
        _ => {}
    }
}
```

Now, whenever a downstream component has an error in its actions, it can simply just set its own error state:

```rust
fn Commandline(cx: Scope) -> Element {
    let set_error = use_set(cx, INPUT_ERROR);

    cx.render(rsx!{
        input {
            oninput: move |evt| {
                if evt.value.len() > 20 {
                    set_error(InputError::TooLong);
                }
            }
        }
    })
}
```

This approach to error handling is best in apps that have "well defined" error states. Consider using a crate like `thiserror` or `anyhow` to simplify the generation of the error types.

This pattern is widely popular in many contexts and is particularly helpful whenever your code generates a non-recoverable error. You can gracefully capture these "global" error states without panicking or mucking up state.
