## Additional Information that may be useful

<details>
<summary>This function is a hook which means you need to <b>follow the rules of hooks</b> when you call it. You can click here to learn more about the rules of hooks.</summary>

Hooks in dioxus need to run in the same order every time you run the component. To make sure you run hooks in a consistent order, you should follow the rules of hooks:

1. Hooks should only be called from the root of a component or another hook

```rust
# use dioxus::prelude::*;
fn App() -> Element {
    // ✅ You can call hooks from the body of a component
    let number = use_signal(|| 1);
    if number() == 1 {
        // ❌ You can run into issues if you can hooks inside other expressions inside your component
        // If number changes from 0 to 1, the order of the hooks will be different and your app may panic
        let string = use_signal(|| "hello world".to_string());
    }

    todo!()
}

fn use_my_hook() -> Signal<i32> {
    // ✅ You can call hooks from the body of other hooks
    let number = use_signal(|| 1);
    // ❌ Again, creating hooks inside expressions inside other hooks can cause issues
    if number() == 1 {
        let string = use_signal(|| "hello world".to_string());
    }

    number
}
```

2. Hooks should always start with `use_` to make it clear that you need to call them in a consistent order

Because hooks should only be called from the root of a component or another hook, you shouldn't call hooks inside of:

- ❌ Conditionals

```rust
# use dioxus::prelude::*;
fn App() -> Element {
    let number = use_signal(|| 1);
    // ❌ Changing the condition will change the order of the hooks
    if number() == 1 {
        let string = use_signal(|| "hello world".to_string());
    }

    // ❌ Changing the value you are matching will change the order of the hooks
    match number() {
        1 => {
            let string = use_signal(|| "hello world".to_string());
        },
        _ => (),
    }

    todo!()
}
```

- ❌ Loops

```rust
# use dioxus::prelude::*;
fn App() -> Element {
    let number = use_signal(|| 1);
    // ❌ Changing the loop will change the order of the hooks
    for i in 0..number() {
        let string = use_signal(|| "hello world".to_string());
    }

    todo!()
}
```

- ❌ Event Handlers

```rust
# use dioxus::prelude::*;
fn App() -> Element {
    rsx! {
        button {
            onclick: move |_| {
                // ❌ Calling the event handler will change the order of the hooks
                use_signal(|| "hello world".to_string());
            },
            "Click me"
        }
    }
}
```

- ❌ Initialization closures in other hooks

```rust
# use dioxus::prelude::*;
fn App() -> Element {
    let number = use_signal(|| {
        // ❌ This closure will only be called when the component is first created. Running the component will change the order of the hooks
        let string = use_signal(|| "hello world".to_string());
        string()
    });

    todo!()
}
```

<details>
<summary>Why do hooks need to run in a consistent order?</summary>

Hooks need to be run in a consistent order because dioxus stores hooks in a list and uses the order you run hooks in to determine what part of the state belongs to which hook.

Lets look at an example component:

```rust
# use dioxus::prelude::*;
fn App() -> Element {
    let number = use_signal(|| 1); // Hook 1
    let string = use_signal(|| "hello world".to_string()); // Hook 2
    let doubled = use_memo(move || number() * 2); // Hook 3

    todo!()
}
```

When we first create the component, we run the hooks in the order they are defined and store the state in the component in a list.

```rust, ignore
[
    Box::new(1),
    Box::new("hello world".to_string()),
    Box::new(2),
]
```

Next time we run the component, we return items from the state list instead of creating state again.

```rust, ignore
[
    Box::new(1), // Hook 1 returns 1
    Box::new("hello world".to_string()), // Hook 2 returns "hello world"
    Box::new(2), // Hook 3 returns 2
]
```

The order the hooks are run it must be the same because the order determines which hook gets what state! If you run the hooks in a different order, dioxus may panic because it can't turn the state back into the right type or you may just get the wrong state for your hook.

</details>

</details>
