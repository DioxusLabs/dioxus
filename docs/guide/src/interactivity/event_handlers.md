# Event handlers

To make our boring UIs less static and more interesting, we want to add the ability to interact to user input. To do this, we need to add some event handlers.


## The most basic events: clicks

If you've poked around in the Dioxus examples at all, you've definitely noticed the support for buttons and clicks. To add some basic action when a button is clicked, we need to define a button and then attach an "onclick" handler to it.

```rust
fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        button {
            onclick: move |evt| println!("I've been clicked!"),
            "click me!"
        }
    })
}
```

If you're using the builder pattern, it's pretty simple too. `onclick` is a method for any builder with HTML elements.

```rust
fn app(cx: Scope) -> Element {
    button(&cx)
        .onclick(move |evt| println!("I've been clicked!"))
        .text("click me!")
        .build()
}
```

The event handler is different in Dioxus than other libraries. Event handlers in Dioxus may borrow any data that has a lifetime that matches the component's scope. This means we can save a value with `use_hook` and then use it in our handler.

```rust
fn app(cx: Scope) -> Element {
    let val = cx.use_hook(|_| 10);

    button(&cx)
        .onclick(move |evt| println!("Current number {val}"))
        .text("click me!")
        .build()
}
```


## The `Event` object

When the listener is fired, Dioxus will pass in any related data through the `event` parameter. This holds helpful state relevant to the event. For instance, on forms, Dioxus will fill in the "values" field.

```rust
// the FormEvent is roughly
struct FormEvent {
    value: String,
    values: HashMap<String, String>
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        form {
            onsubmit: move |evt| {
                println!("Values of form are {evt.values:#?}");
            }
            input { id: "password", name: "password" }
            input { id: "username", name: "username" }
        }
    })
}
```

## Stopping propagation

With a complex enough UI, you might realize that listeners can actually be nested.

```rust
div {
    onclick: move |evt| {},
    "outer",
    div {
        onclick: move |evt| {},
        "inner"
    }
}
```

In this particular layout, a click on the inner div is transitively also a click on the outer div. If we didn't want the outer div to be triggered every time we trigger the inner div, then we'd want to call "cancel_bubble".

This will prevent any listeners above the current listener from being triggered.

```rust
div {
    onclick: move |evt| {},
    "outer",
    div {
        onclick: move |evt| {
            // now, outer won't be triggered
            evt.cancel_bubble();
        },
        "inner"
    }
}
```

## Prevent Default

With HTML based renderers, the browser will automatically perform some action. For text inputs, this would be entering the provided key. For forms, this might involve navigating the page.

In some instances, you don't want this default behavior. In these cases, instead of handling the event directly, you'd want to prevent any default handlers.

Normally, in React or JavaScript, you'd call "preventDefault" on the event in the callback. Dioxus does *not* currently support this behavior. Instead, you need to add an attribute to the element generating the event.

```rust
form {
    prevent_default: "onclick",
    onclick: move |_|{
        // handle the event without navigating the page.
    }
}
```
