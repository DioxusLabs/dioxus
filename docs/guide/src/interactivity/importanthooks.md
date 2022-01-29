# `use_state` and `use_ref`

Most components you will write in Dioxus will need to store state somehow. For local state, we provide two very convenient hooks:

- `use_state`
- `use_ref`

Both of these hooks are extremely powerful and flexible, so we've dedicated this section to understanding them properly.

> These two hooks are not the only way to store state. You can always build your own hooks!

## Note on Hooks

If you're struggling with errors due to usage in hooks, make sure you're following the rules of hooks:

- Functions with "use_" should not be called in callbacks
- Functions with "use_" should not be called out of order
- Functions with "use_" should not be called in loops or conditionals

A large majority of issues that seem to be "wrong with Dioxus" are actually just a misuse of hooks.

## `use_state`

The `use_state` hook is very similar to its React counterpart. When we use it, we get two values: 

- The value itself as an `&T`
- A handle to set the value `&UseState<T>`

```rust
let (count, set_count) = use_state(&cx, || 0);

// then to set the count
set_count(count + 1)
```

However, the `set_count` object is more powerful than it looks. You can use it as a closure, but you can also call methods on it to do more powerful operations.

For instance, we can get a handle to the current value at any time:

```rust
let current: Rc<T> = set_count.current();
```

Or, we can get the inner handle to set the value

```rust
let setter: Rc<dyn Fn(T)> = set_count.setter();
```

Or, we can set a new value using the current value as a reference:

```rust
set_count.modify(|i| i + 1);
```

If the value is cheaply cloneable, then we can call `with_mut` to get a mutable reference to the value:

```rust
set_count.with_mut(|i| *i += 1);
```

Alternatively, we can call `make_mut` which gives us a `RefMut` to the underlying value:

```rust
*set_count.make_mut() += 1;
```

Plus, the `set_count` handle is cloneable into async contexts, so we can use it in futures.

```rust
// count up infinitely
cx.spawn({
    to_owned![set_count]; 
    async move {
        loop {
            wait_ms(100).await;
            set_count.modify(|i| i + 1);
        }
    }
})
```

## `use_ref`

You might've noticed a fundamental limitation to `use_state`: to modify the value in-place, it must be cheaply cloneable. But what if your type is not cheap to clone?

In these cases, you should reach for `use_ref` which is essentially just a glorified `Rc<RefCell<T>>` (typical Rust UI shenanigans).

This provides us some runtime locks around our data, trading reliability for performance. For most cases though, you will find it hard to make `use_ref` crash.

> Note: this is *not* exactly like its React counterpart since calling `write` will cause a re-render. For more React parity, use the `write_silent` method.

To use the hook:

```rust
let names = use_ref(&cx, || vec!["susie", "calvin"]);
```

To read the hook values, use the `read()` method:

```rust
cx.render(rsx!{
    ul {
        names.read().iter().map(|name| {
            rsx!{ "{name}" }
        })
    }
})
```

And then to write into the hook value, use the `write` method:

```rust
names.write().push("Tiger");
```

If you don't want to re-render the component when names is updated, then we can use the `write_silent` method:

```rust
names.write().push("Transmogrifier");
```

Again, like `UseState`, the `UseRef` handle is clonable into async contexts:


```rust
// infinitely push calvin into the list
cx.spawn({
    to_owned![names]; 
    async move {
        loop {
            wait_ms(100).await;
            names.write().push("Calvin");
        }
    }
})
```


## Wrapping up

These two hooks are extremely powerful at storing state.
