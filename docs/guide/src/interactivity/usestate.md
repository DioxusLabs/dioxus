# Fundamental hook: `use_state`

The first fundamental hook for state management is `use_state`. This particular hook is designed to work well with the entire Dioxus ecosystem including futures, children, and memoization.


## Basic usage.

The simplest use case of `use_state` is a simple counter. The handle returned by `use_state` implements `Add` and `AddAssign`. Writing through `AddAssign` will automatically mark the component as dirty, forcing an update.

```rust
fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 0);

    rsx!(cx, button { onclick: move |_| count += 1, })
}
```

## Reference

### Common ops

The `use_state` hook is very similar to its React counterpart. When we use it, we get a smart pointer - essentially an `Rc<T>` that tracks when we update it.

```rust
let mut count = use_state(&cx, || 0);

// then to set the count
count += 1;
```


### Current value

Our `count` value is more than just an integer. Because it's a smart pointer, it also implements other useful methods helpful in various contexts.

For instance, we can get a handle to the current value at any time:

```rust
let current: Rc<T> = count.current();
```

Or, we can get the inner handle to set the value:

```rust
let setter: Rc<dyn Fn(T)> = count.setter();
```

### Modify

Or, we can set a new value using the current value as a reference:

```rust
count.modify(|i| i + 1);
```

### `with_mut` and `make_mut`

If the value is cheaply cloneable, then we can call `with_mut` to get a mutable reference to the value:

```rust
count.with_mut(|i| *i += 1);
```

Alternatively, we can call `make_mut` which gives us a `RefMut` to the underlying value:

```rust
*count.make_mut() += 1;
```

### Use in Async

Plus, the `set_count` handle is cloneable into async contexts, so we can use it in futures.

```rust
// count up infinitely
cx.spawn({
    let count = count.clone();
    async move {
        loop {
            wait_ms(100).await;
            count += 1;
        }
    }
})
```

## Using UseState for simple data

The best use case of `use_state` is to manage "simple" data local to your component. This should be things like numbers, strings, small maps, and cheaply-clonable types.

```rust
let val = use_state(&cx, || 0);
let val = use_state(&cx, || "Hello!");
let val = use_state(&cx, || vec![1, 3, 3, 7]);
```

UseState can be sent down into child components too.

You can either pass by reference to always force the child to update when the value changes, or you can clone the handle to take advantage of automatic memoization. In these cases, calling "get" will return stale data - always prefer "current" when "cloning" the UseState. This automatic memoization of UseState solves a performance problem common in React.

```rust

fn app(cx: Scope) -> Element {
    let val = use_state(&cx, || 0);

    cx.render(rsx!{
        Child { val: val.clone() }
    })
}

fn Child(cx: Scope, val: UseState<i32>) -> Element {
    // ...
}
```
