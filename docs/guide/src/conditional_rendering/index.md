# Conditional Rendering

Sometimes you want to render different things depending on the state/props. With Dioxus, just describe what you want to see – the framework will take care of making the necessary changes on the fly if the state or props change!

```rust
{{#include ../../examples/conditional_rendering.rs:if_else}}
```

## Rendering Nothing

To render nothing, you can return `None` from a component. This is useful if you want to conditionally hide something:

```rust
{{#include ../../examples/conditional_rendering.rs:conditional_none}}
```

## Rendering Lists

