# Component Children

In some cases, you may wish to create a component that acts as a container for some other content, without the component needing to know what that content is. To achieve this, create a prop of type `Element`:

```rust, no_run
{{#include ../../../examples/component_element_props.rs:Clickable}}
```

Then, when rendering the component, you can pass in the output of `cx.render(rsx!(...))`:

```rust, no_run
{{#include ../../../examples/component_element_props.rs:Clickable_usage}}
```

> Note: Since `Element<'a>` is a borrowed prop, there will be no memoization.

> Warning: While it may compile, do not include the same `Element` more than once in the RSX. The resulting behavior is unspecified.

## The `children` field

Rather than passing the RSX through a regular prop, you may wish to accept children similarly to how elements can have children. The "magic" `children` prop lets you achieve this:

```rust, no_run
{{#include ../../../examples/component_children.rs:Clickable}}
```

This makes using the component much simpler: simply put the RSX inside the `{}` brackets – and there is no need for a `render` call or another macro!

```rust, no_run
{{#include ../../../examples/component_children.rs:Clickable_usage}}
```
