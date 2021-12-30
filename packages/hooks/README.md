# Common hooks for Dioxus

This crate includes some basic useful hooks for dioxus:

- use_state
- use_ref
- use_future
- use_coroutine

## use_state

The primary mechanism of stored state.

You can always use it "normally" with the `split` method:

```rust
// Rusty-smart-pointer usage:
let value = use_state(&cx, || 10);

// "Classic" usage:
let (value, set_value) = use_state(&cx, || 0).split();
```
