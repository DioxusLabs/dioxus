# Event Handlers

Event Handlers let you react to user input in your application. In Dioxus, event handlers accept a closure that is called when the event occurs:

```rust, no_run
use dioxus::prelude::*;

fn App() -> Element {
    rsx! {
        button {
            // The `onclick` event accepts a closure with the signature `fn(Event)`
            onclick: |event_data| println!("clicked! I got the event data: {event_data:?}"),
            "Click me"
        }
    }
}
```

## Event Lifetimes

Events take a closure with the `'static` lifetime. This means that the closure can only access data that either exists for the entire lifetime of the application, or data that you move into the closure.

State in dioxus is `copy` which makes it very easy to move into `'static` closures like event handlers:

```rust, no_run
# use dioxus::prelude::*;
let mut count = use_signal(|| 0);

rsx! {
    button {
        // Since we added the `move` keyword, the closure will move the `count` signal into the closure
        onclick: move |_| {
            // This will panic because the `count` signal is not in scope
            count.set(count() + 1);
        },
        "Click me"
    }
};
```

If you need to access data that is not `Copy`, you may need to clone the data before you move it into the closure:

```rust, no_run
# use dioxus::prelude::*;
// String is not `Copy`
let string = "hello world".to_string();

rsx! {
    button {
        // The string only has one owner. We could move it into this closure, but since we want to use the string in other closures later, we will clone it instead
        onclick: {
            // Clone the string in a new block
            let string = string.clone();
            // Then move the cloned string into the closure
            move |_| println!("{}", string)
        },
        "Print hello world"
    }
    button {
        // We don't use the string after this closure, so we can just move it into the closure directly
        onclick: move |_| println!("{}", string),
        "Print hello world again"
    }
};
```

## Async Event Handlers

In addition to closures that return nothing, you can also use async closures to handle events. If you return an async block from an event handler, dioxus will automatically spawn it:

```rust, no_run
use dioxus::prelude::*;

fn App() -> Element {
    rsx! {
        button {
            // The `onclick` event can also accept a closure that returns an async block
            onclick: move |_| async move {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                println!("You clicked the button one second ago!");
            },
            "Click me"
        }
    }
}
```
