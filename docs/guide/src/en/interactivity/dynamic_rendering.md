# Dynamic Rendering

Sometimes you want to render different things depending on the state/props. With Dioxus, just describe what you want to see using Rust control flow – the framework will take care of making the necessary changes on the fly if the state or props change!

## Conditional Rendering

To render different elements based on a condition, you could use an `if-else` statement:

```rust
{{#include ../../../examples/conditional_rendering.rs:if_else}}
```

> You could also use `match` statements, or any Rust function to conditionally render different things.

### Improving the `if-else` Example

You may have noticed some repeated code in the `if-else` example above. Repeating code like this is both bad for maintainability and performance. Dioxus will skip diffing static elements like the button, but when switching between multiple `rsx` calls it cannot perform this optimization. For this example either approach is fine, but for components with large parts that are reused between conditionals, it can be more of an issue.

We can improve this example by splitting up the dynamic parts and inserting them where they are needed.

```rust
{{#include ../../../examples/conditional_rendering.rs:if_else_improved}}
```

### Inspecting `Element` props

Since `Element` is a `Option<VNode>`, components accepting `Element` as a prop can inspect its contents, and render different things based on that. Example:

```rust
{{#include ../../../examples/component_children_inspect.rs:Clickable}}
```

You can't mutate the `Element`, but if you need a modified version of it, you can construct a new one based on its attributes/children/etc.


## Rendering Nothing

To render nothing, you can return `None` from a component. This is useful if you want to conditionally hide something:

```rust
{{#include ../../../examples/conditional_rendering.rs:conditional_none}}
```

This works because the `Element` type is just an alias for `Option<VNode>`

> Again, you may use a different method to conditionally return `None`. For example the boolean's [`then()`](https://doc.rust-lang.org/std/primitive.bool.html#method.then) function could be used.

## Rendering Lists

Often, you'll want to render a collection of components. For example, you might want to render a list of all comments on a post.

For this, Dioxus accepts iterators that produce `Element`s. So we need to:

- Get an iterator over all of our items (e.g., if you have a `Vec` of comments, iterate over it with `iter()`)
- `.map` the iterator to convert each item into a `LazyNode` using `rsx!(...)`
  - Add a unique `key` attribute to each iterator item
- Include this iterator in the final RSX (or use it inline)

Example: suppose you have a list of comments you want to render. Then, you can render them like this:

```rust
{{#include ../../../examples/rendering_lists.rs:render_list}}
```

### Inline for loops

Because of how common it is to render a list of items, Dioxus provides a shorthand for this. Instead of using `.iter, `.map`, and `rsx`, you can use a `for` loop with a body of rsx code:

```rust
{{#include ../../../examples/rendering_lists.rs:render_list_for_loop}}
```

### The `key` Attribute

Every time you re-render your list, Dioxus needs to keep track of which items go where to determine what updates need to be made to the UI.

For example, suppose the `CommentComponent` had some state – e.g. a field where the user typed in a reply. If the order of comments suddenly changes, Dioxus needs to correctly associate that state with the same comment – otherwise, the user will end up replying to a different comment!

To help Dioxus keep track of list items, we need to associate each item with a unique key. In the example above, we dynamically generated the unique key. In real applications, it's more likely that the key will come from e.g. a database ID. It doesn't matter where you get the key from, as long as it meets the requirements:

- Keys must be unique in a list
- The same item should always get associated with the same key
- Keys should be relatively small (i.e. converting the entire Comment structure to a String would be a pretty bad key) so they can be compared efficiently

You might be tempted to use an item's index in the list as its key. That’s what Dioxus will use if you don’t specify a key at all. This is only acceptable if you can guarantee that the list is constant – i.e., no re-ordering, additions, or deletions.

> Note that if you pass the key to a component you've made, it won't receive the key as a prop. It’s only used as a hint by Dioxus itself. If your component needs an ID, you have to pass it as a separate prop.
