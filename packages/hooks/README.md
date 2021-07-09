# Common hooks for Dioxus

This crate includes some basic useful hooks for dioxus:

- use_state
- use_ref
- use_collection
- use_task
- use_signal

## use_state

The king daddy of state hooks.

You can always use it "normally" with the `split` method:

```rust
// Normal usage:
let value = use_state(cx, || 10);

// "Classic" usage:
let (value, set_value) = use_state(cx, || 0).classic();
```

## use_ref


## use_rwlock
A multithreaded form of RwLock for use in tasks
```rust
let val = use_rwlock(cx, || 10);
use_task((), || async loop {
    *val.write().unwrap() += 1;
    async_std::task::delay(Duration::from_ms(1000)).await;
});
use_task((), || async loop {
    *val.write().unwrap() -= 1;
    async_std::task::delay(Duration::from_ms(500)).await;
});
```

## use_hashmap
Store a memoized collection with similar semantics to use_state. Comes with a bunch of utility methods to make working with collections easier. Is essentially a wrapper over the immutable hashmap in im-rc.

```rust
let todos = use_hashmap(cx, |map| map.insert("bob", "bill"));
cx.render(rsx!(
    button { onclick: move |_| todos.insert("bob", "bill")
        "add random todo"
    }
)

```

## use_task

use_task submits a task to the dioxus task queue to be progressed during Dioxus's async event loop. The task must not return anything


## use_signal

