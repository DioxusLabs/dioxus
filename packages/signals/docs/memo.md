Memos are the result of computing a value from `use_memo`.

You may have noticed that this struct doesn't have many methods. Most methods for `Memo` are defined on the [`Readable`] and [`Writable`] traits.

# Reading a Memo

You can use the methods on the `Readable` trait to read a memo:

```rust, no_run
# use dioxus::prelude::*;

fn app() -> Element {
    let mut count = use_signal(|| 0);
    // The memo will rerun any time we write to the count signal
    let halved = use_memo(move || count() / 2);

    rsx! {
        // When we read the value of memo, the current component will subscribe to the result of the memo. It will only rerun when the result of the memo changes.
        "{halved}"
        button {
            onclick: move |_| {
                count += 1;
            },
            "Increment"
        }
    }
}
```

Memo also includes helper methods just like [`Signal`]s to make it easier to use. Calling a memo like a function will clone the inner value:

```rust, no_run
# use dioxus::prelude::*;

fn app() -> Element {
    let mut count = use_signal(|| 0);
    // The memo will rerun any time we write to the count signal
    let halved = use_memo(move || count() / 2);
    // This will rerun any time the halved value changes
    let doubled = use_memo(move || 2 * halved());

    rsx! {
        "{doubled}"
        button {
            onclick: move |_| {
                count += 1;
            },
            "Increment"
        }
    }
}
```

For a full list of all the helpers available, check out the [`Readable`], [`ReadableVecExt`], and [`ReadableOptionExt`] traits.

# Memos with Async

Because Memos check borrows at runtime, you need to be careful when reading memos inside of async code. If you hold a read of a memo over an await point, that read may still be open when the memo reruns which will cause a panic:

```rust, no_run
# use dioxus::prelude::*;
# async fn sleep(delay: u32) {}
async fn double_me_async(value: &u32) -> u32 {
    sleep(100).await;
    *value * 2
}
let mut signal = use_signal(|| 0);
let halved = use_memo(move || signal() / 2);

let doubled = use_resource(move || async move {
    // Don't hold reads over await points
    let halved = halved.read();
    // While the future is waiting for the async work to finish, the read will be open
    double_me_async(&halved).await
});

rsx!{
    "{doubled:?}"
    button {
        onclick: move |_| {
            // When you write to signal, it will cause the memo to rerun which may panic because you are holding a read of the memo over an await point
            signal += 1;
        },
        "Increment"
    }
};
```

Instead of holding a read over an await point, you can clone whatever values you need out of your memo:

```rust, no_run
# use dioxus::prelude::*;
# async fn sleep(delay: u32) {}
async fn double_me_async(value: u32) -> u32 {
    sleep(100).await;
    value * 2
}
let mut signal = use_signal(|| 0);
let halved = use_memo(move || signal() / 2);

let doubled = use_resource(move || async move {
    // Calling the memo will clone the inner value
    let halved = halved();
    double_me_async(halved).await;
});

rsx!{
    "{doubled:?}"
    button {
        onclick: move |_| {
            signal += 1;
        },
        "Increment"
    }
};
```

# Memo lifecycle

Memos are implemented with [generational-box](https://crates.io/crates/generational-box) which makes all values Copy even if the inner value is not Copy.

This is incredibly convenient for UI development, but it does come with some tradeoffs. The lifetime of the memo is tied to the lifetime of the component it was created in. If you drop the component that created the memo, the memo will be dropped as well. You might run into this if you try to pass a memo from a child component to a parent component and drop the child component. To avoid this you can create your memo higher up in your component tree, or use global memos.

TLDR **Don't pass memos up in the component tree**. It will cause issues:

```rust
# use dioxus::prelude::*;
fn MyComponent() -> Element {
    let child_signal = use_signal(|| None);

    rsx! {
        IncrementButton {
            child_signal
        }
    }
}

#[component]
fn IncrementButton(mut child_signal: Signal<Option<Memo<i32>>>) -> Element {
    let signal_owned_by_child = use_signal(|| 0);
    let memo_owned_by_child = use_memo(move || signal_owned_by_child() * 2);
    // Don't do this: it may cause issues if you drop the child component
    child_signal.set(Some(memo_owned_by_child));

    todo!()
}
```
