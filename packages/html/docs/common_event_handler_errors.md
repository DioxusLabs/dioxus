## Compiler errors you may run into while using event handlers

<details>
<summary>function requires argument type to outlive `'static`</summary>

Event handler in Dioxus need only access data that can last for the entire lifetime of the application. That generally means data that is moved into the closure. **If you get this error, you may have forgotten to add `move` to your closure.**

Broken component:

```rust, compile_fail
# use dioxus::prelude::*;
// We return an Element which can last as long as the component is on the screen
fn App() -> Element {
    // Signals are `Copy` which makes them very easy to move into `'static` closures like event handlers
    let state = use_signal(|| "hello world".to_string());

    rsx! {
        button {
            // ❌ Without `move`, rust will try to borrow the `state` signal which fails because the state signal is dropped at the end of the function
            onclick: |_| {
                println!("You clicked the button! The state is: {state}");
            },
            "Click me"
        }
    }
    // The state signal is dropped here, but the event handler still needs to access it
}
```

Fixed component:

```rust, no_run
# use dioxus::prelude::*;
fn App() -> Element {
    let state = use_signal(|| "hello world".to_string());

    rsx! {
        button {
            // ✅ The `move` keyword tells rust it can move the `state` signal into the closure. Since the closure owns the signal state, it can read it even after the function returns
            onclick: move |_| {
                println!("You clicked the button! The state is: {state}");
            },
            "Click me"
        }
    }
}
```

</details>

<details>
<summary>use of moved value: `your_value` value used here after move</summary>

Data in rust has a single owner. If you run into this error, you have likely tried to move data that isn't `Copy` into two different closures. **You can fix this issue by making your data `Copy` or calling `clone` on it before you move it into the closure.**

Broken component:

```rust, compile_fail
# use dioxus::prelude::*;
// `MyComponent` accepts a string which cannot be copied implicitly
#[component]
fn MyComponent(string: String) -> Element {
    rsx! {
        button {
            // ❌ We are moving the string into the onclick handler which means we can't access it elsewhere
            onclick: move |_| {
                println!("{string}");
            },
            "Print hello world"
        }
        button {
            // ❌ Since we already moved the string, we can't move it into the onclick handler again. This will cause a compiler error
            onclick: move |_| {
                println!("{string}");
            },
            "Print hello world again"
        }
    }
}
```

You can fix this issue by either:

- Making your data `Copy` with `ReadOnlySignal`:

```rust, no_run
# use dioxus::prelude::*;
// `MyComponent` accepts `ReadOnlySignal<String>` which implements `Copy`
#[component]
fn MyComponent(string: ReadOnlySignal<String>) -> Element {
    rsx! {
        button {
            // ✅ Because the `string` signal is `Copy`, we can copy it into the closure while still having access to it elsewhere
            onclick: move |_| println!("{}", string),
            "Print hello world"
        }
        button {
            // ✅ Since `string` is `Copy`, we can move it into the onclick handler again
            onclick: move |_| println!("{}", string),
            "Print hello world again"
        }
    }
}
```

- Calling `clone` on your data before you move it into the closure:

```rust, no_run
# use dioxus::prelude::*;
// `MyComponent` accepts a string which doesn't implement `Copy`
#[component]
fn MyComponent(string: String) -> Element {
    rsx! {
        button {
            // ✅ The string only has one owner. We could move it into this closure, but since we want to use the string in other closures later, we will clone it instead
            onclick: {
                // Clone the string in a new block
                let string = string.clone();
                // Then move the cloned string into the closure
                move |_| println!("{}", string)
            },
            "Print hello world"
        }
        button {
            // ✅ We don't use the string after this closure, so we can just move it into the closure directly
            onclick: move |_| println!("{}", string),
            "Print hello world again"
        }
    }
}
```

</details>
