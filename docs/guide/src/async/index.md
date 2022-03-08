# Working with Async

Not all apps you'll build can be self-contained with synchronous code. You'll often need to interact with file systems, network interfaces, hardware, or timers.

So far, we've only talked about building apps with synchronous code, so this chapter will focus integrating asynchronous code into your app.

## The Runtime

By default, Dioxus-Desktop ships with the `Tokio` runtime and automatically sets everything up for you. This is currently not configurable, though it would be easy to write an integration for Dioxus desktop that uses a different asynchronous runtime.

## Send/Sync
Writing apps that deal with Send/Sync can be frustrating at times. Under the hood, Dioxus is not currently thread-safe, so any async code you write does *not* need to be `Send/Sync`. That means that you can use non-thread-safe structures like `Cell`, `Rc`, and `RefCell`.

All async code in your app is polled on a `LocalSet`, so you can also use `tokio::spawn_local`.

## Spawning a future

Currently, all futures in Dioxus must be `'static`. To spawn a future, simply call `cx.spawn()`.

```rust
rsx!{
    button {
        onclick: move |_| cx.spawn(async move {
            let result = fetch_thing().await;
        })
    }
}
```

Calling `spawn` will give you a `JoinHandle` which lets you cancel or pause the future.


## Setting state from within a future

If you start to write some async code, you'll quickly be greeted with the infamous error about borrowed items in static closures.

```
error[E0759]: `cx` has an anonymous lifetime `'_` but it needs to satisfy a `'static` lifetime requirement
  --> examples/tasks.rs:13:27
   |
12 | fn app(cx: Scope) -> Element {
   |            ----- this data with an anonymous lifetime `'_`...
13 |     let count = use_state(&cx, || 0);
   |                           ^^^ ...is used here...
14 |
15 |     use_future(&cx, (), move |_| {
   |     ---------- ...and is required to live as long as `'static` here
   |
note: `'static` lifetime requirement introduced by this bound
  --> /Users/jonkelley/Development/dioxus/packages/hooks/src/usefuture.rs:25:29
   |
25 |     F: Future<Output = T> + 'static,
   |                             ^^^^^^^

For more information about this error, try `rustc --explain E0759`.
error: could not compile `dioxus` due to previous error
```

Rustc tends to provide great feedback in its errors, but this particular error is actually quite unhelpful. For reference, here's our code:

```rust
fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 0);

    cx.spawn(async move {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        count += 1;
    });

    cx.render(rsx! {
        button {
            onclick: move |_| count.set(0),
            "Reset the count"
        }
    })
}
```

Simple, right? We spawn a future that updates the value after one second has passed. Well, yes, and no. Our `count` value is only valid for the lifetime of this component, but our future could still be running even after the component re-renders. By default, Dioxus places a requirement on all futures to be `'static`, meaning they can't just borrow state from our hooks outright.

To get around this, we need to get a `'static` handle to our state. All Dioxus hooks implement `Clone`, so you simply need to call clone before spawning your future. Any time you get the particular error above, make sure you call `Clone` or `ToOwned`.

```rust
cx.spawn({
    let mut count = count.clone();
    async move {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        count += 1;
    }
});
```

To make this a little bit easier, Dioxus exports the `to_owned!` macro which will create a binding as shown above, which can be quite helpful when dealing with many values.

```rust
cx.spawn({
    to_owned![count, age, name, description, etc];
    async move {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        count += 1;
    }
});
```

