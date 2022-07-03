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

Often, you'll want to render a collection of components. For example, you might want to render a list of all comments on a post.

For this, Dioxus accepts iterators that produce `Element`s. So we need to:

- Get an iterator over all of our items (e.g., if you have a `Vec` of comments, iterate over it with `iter()`)
- `.map` the iterator to convert each item into a rendered `Element` using `cx.render(rsx!(...))`
- Include this iterator in the final RSX

Example: suppose you have a list of comments you want to render. Then, you can render them like this:

```rust
{{#include ../../examples/rendering_lists.rs:render_list}}
```