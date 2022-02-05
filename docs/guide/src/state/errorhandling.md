# Error handling


Astute observers might have noticed that `Element` is actually a type alias for `Option<VNode>`. You don't need to know what a `VNode` is, but it's important to recognize that we could actually return nothing at all:

```rust
fn App((cx, props): Component) -> Element {
    None
}
```

> This section is currently under construction! 🏗
