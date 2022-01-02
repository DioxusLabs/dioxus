# Tasks

All async code in Dioxus must be explicit and handled through Dioxus' task system.

In this chapter, we'll learn how to spawn new tasks through our `Scope`.

## Spawning a task

You can push any `'static` future into the Dioxus future queue by simply calling `cx.spawn` to spawn a task. Pushing a future returns a `TaskId` which can then be used to cancel the 

```rust
fn App(cx: Scope) -> Element {
    cx.spawn(async {
        let mut count = 0;
        loop {
            tokio::time::delay(std::instant::Duration::from_millis(500)).await;
            count += 1;
            println!("Current count is {}", count);
        }
    });

    None
}
```

The future must be `'static` - so any values captured by the task must not carry any references to `cx`. All the Dioxus hooks have a method called `for_async` which will create a slightly more limited handle to the hook for you to use in your async code.

```rust
fn App(cx: Scope) -> Element {
    let mut count = use_state(&cx, || 0);

    let taskid = cx.spawn({
        let mut count = count.for_async();
        async {
            loop {
                tokio::time::delay(std::instant::Duration::from_millis(500)).await;
                count += 1;
                println!("Current count is {}", count);
            }
        }
    });
}
```

The task will run in the background until it is completed.

> Note: `spawn` will always spawn a *new* future. You probably want to call it from a hook initializer instead of the main body of your component.

When bringing lots of values into your task, we provide the `for_async!` macro which will can `for_async` on all values passed in. For types that implement `ToOwned`, `for_async!` will simply call `ToOwned` for that value.

```rust
fn App(cx: Scope) -> Element {
    let mut age = use_state(&cx, || 0);
    let mut name = use_state(&cx, || "Bob");
    let mut description = use_state(&cx, || "asd".to_string());

    let taskid = cx.spawn({
        for_async![count, name, description]
        async { /* code that uses count/name/description */ }
    });
}
```

## Details of Tasks

Calling `spawn` is *not* a hook and will *always* generate a new task. Make sure to only spawn tasks when you want to. You should *probably* not call `spawn` in the main body of your component, since a new task will be spawned on every render.

## Spawning Tokio Tasks (for multithreaded use cases)

Sometimes, you might want to spawn a background task that needs multiple threads or talk to hardware that might block your app code. In these cases, we can can directly spawn a `tokio task` from our future. For Dioxus-Desktop, your task will be spawned onto Tokio's Multithreaded runtime:

```rust
cx.spawn({
    tokio::spawn(async {
        // some multithreaded work
    }).await;

    tokio::spawn_blocking(|| {
        // some extremely blocking work
    }).await;

    tokio::spawn_local(|| {
        // some !Send work
    }).await;
})
```

> Note: Tokio tasks must be `Send`. Most hooks are `Send` compatible, but if they aren't, then you can use `spawn_local` to spawn onto Dioxus-Desktop's `localset`.


