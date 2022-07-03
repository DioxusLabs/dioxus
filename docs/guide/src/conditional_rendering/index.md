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
  - Add a unique `key` attribute to each iterator item
- Include this iterator in the final RSX

Example: suppose you have a list of comments you want to render. Then, you can render them like this:

```rust
{{#include ../../examples/rendering_lists.rs:render_list}}
```

### The `key` Attribute

Every time you re-render your list, Dioxus needs to keep track of which item went where, because the order of items in a list might change – items might be added, removed or swapped. Despite that, Dioxus needs to:

- Keep track of component state
- Efficiently figure out what updates need to be made to the UI

For example, suppose the `CommentComponent` had some state – e.g. a field where the user typed in a reply. If the order of comments suddenly changes, Dioxus needs to correctly associate that state with the same comment – otherwise, the user will end up replying to a different comment!

To help Dioxus keep track of list items, we need to associate each item with a unique key. In the example above, we dynamically generated the unique key. In real applications, it's more likely that the key will come from e.g. a database ID. It doesn't really matter where you get the key from, as long as it meets the requirements

- Keys must be unique in a list
- The same item should always get associated with the same key
- Keys should be relatively small (i.e. converting the entire Comment structure to a String would be a pretty bad key) so they can be compared efficiently
