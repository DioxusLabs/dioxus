## Compiler errors you may run into while using spawn

<details>
<summary>async block may outlive the current function, but it borrows `value`, which is owned by the current function</summary>

Tasks in Dioxus need only access data that can last for the entire lifetime of the application. That generally means data that is moved into the async block. **If you get this error, you may have forgotten to add `move` to your async block.**

Broken component:

```rust, compile_fail
use dioxus::prelude::*;

fn App() -> Element {
    let signal = use_signal(|| 0);

    use_hook(move || {
        // ❌ The task may run at any point and reads the value of the signal, but the signal is dropped at the end of the function
        spawn(async {
            println!("{}", signal());
        })
    });

    todo!()
}
```

Fixed component:

```rust, no_run
use dioxus::prelude::*;

fn App() -> Element {
    let signal = use_signal(|| 0);

    use_hook(move || {
        // ✅ The `move` keyword tells rust it can move the `state` signal into the async block. Since the async block owns the signal state, it can read it even after the function returns
        spawn(async move {
            println!("{}", signal());
        })
    });

    todo!()
}
```

</details>

<details>
<summary>use of moved value: `value`. move occurs because `value` has type `YourType`, which does not implement the `Copy` trait</summary>

Data in rust has a single owner. If you run into this error, you have likely tried to move data that isn't `Copy` into two different async tasks. **You can fix this issue by making your data `Copy` or calling `clone` on it before you move it into the async block.**

Broken component:

```rust, compile_fail
# use dioxus::prelude::*;
// `MyComponent` accepts a string which cannot be copied implicitly
#[component]
fn MyComponent(string: String) -> Element {
    use_hook(move || {
        // ❌ We are moving the string into the async task which means we can't access it elsewhere
        spawn(async move {
            println!("{}", string);
        });
        // ❌ Since we already moved the string, we can't move it into our new task. This will cause a compiler error
        spawn(async move {
            println!("{}", string);
        })
    });

    todo!()
}
```

You can fix this issue by either:

- Making your data `Copy` with `ReadOnlySignal`:

```rust, no_run
# use dioxus::prelude::*;
// `MyComponent` accepts `ReadOnlySignal<String>` which implements `Copy`
#[component]
fn MyComponent(string: ReadOnlySignal<String>) -> Element {
    use_hook(move || {
        // ✅ Because the `string` signal is `Copy`, we can copy it into the async task while still having access to it elsewhere
        spawn(async move {
            println!("{}", string);
        });
        // ✅ Since `string` is `Copy`, we can copy it into another async task
        spawn(async move {
            println!("{}", string);
        })
    });

    todo!()
}
```

- Calling `clone` on your data before you move it into the closure:

```rust, no_run
# use dioxus::prelude::*;
// `MyComponent` accepts a string which doesn't implement `Copy`
#[component]
fn MyComponent(string: String) -> Element {
    use_hook(move || {
        // ✅ The string only has one owner. We could move it into this closure, but since we want to use the string in other closures later, we will clone it instead
        spawn({
            // Clone the string in a new block
            let string = string.clone();
            // Then move the cloned string into the async block
            async move {
                println!("{}", string);
            }
        });
        // ✅ We don't use the string after this closure, so we can just move it into the closure directly
        spawn(async move {
            println!("{}", string);
        })
    });

    todo!()
}
```

</details>
