# UseFuture

[`use_future`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_future.html) lets you run an async closure, and provides you with its result.

For example, we can make an API request (using [reqwest](https://docs.rs/reqwest/latest/reqwest/index.html)) inside `use_future`:

```rust
{{#include ../../../examples/use_future.rs:use_future}}
```

The code inside `use_future` will be submitted to the Dioxus scheduler once the component has rendered.

We can use `.value()` to get the result of the future. On the first run, since there's no data ready when the component loads, its value will be `None`.  However, once the future is finished, the component will be re-rendered and the value will now be `Some(...)`, containing the return value of the closure.

We can then render that result:

```rust
{{#include ../../../examples/use_future.rs:render}}
```


## Restarting the Future

The `UseFuture` handle provides a `restart` method. It can be used to execute the future again, producing a new value.

## Dependencies

Often, you will need to run the future again every time some value (e.g. a prop) changes. Rather than calling `restart` manually, you can provide a tuple of "dependencies" to the hook. It will automatically re-run the future when any of those dependencies change. Example:


```rust
{{#include ../../../examples/use_future.rs:dependency}}
```
