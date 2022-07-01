# Component Children

In some cases, you may wish to create a component that acts as a container for some other content, without the component needing to know what that content is. To achieve this, create a prop of type `Element`:

```rust
{{#include ../../examples/component_element_props.rs:Clickable}}
```

Then, when rendering the component, you can pass in the output of `cx.render(rsx!(...))`:

```rust
{{#include ../../examples/component_element_props.rs:Clickable_usage}}
```

> Note: Since `Element<'a>` is a borrowed prop, there will be no memoization.

> Warning: While it may compile, do not include the same `Element` more than once in the RSX. The resulting behavior is unspecified.

## The `children` field

Rather than passing the RSX through a regular prop, you may wish to accept children similarly to how elements can have children. The "magic" `children` prop lets you achieve this:

```rust
{{#include ../../examples/component_children.rs:Clickable}}
```

This makes using the component much simpler: simply put the RSX inside the `{}` brackets – and there is no need for a `render` call or another macro!

```rust
{{#include ../../examples/component_children.rs:Clickable_usage}}
```

## Inspecting the `Element`

Since `Element` is a `Option<VNode>`, we can actually inspect the contents of `children`, and render different things based on that. Example:

```rust
{{#include ../../examples/component_children_inspect.rs:Clickable}}
```

You can't mutate the `Element`, but if you need a modified version of it, you can construct a new one based on its attributes/children/etc.

<!-- ## Passing attributes

In the cases where you need to pass arbitrary element properties into a component - say to add more functionality to the `<a>` tag, Dioxus will accept any quoted fields. This is similar to adding arbitrary fields to regular elements using quotes.

```rust

rsx!(
    Clickable {
        "class": "blue-button",
        "style": "background: red;"
    }
)

```

For a component to accept these attributes, you must add an `attributes` field to your component's properties. We can use the spread syntax to add these attributes to whatever nodes are in our component.

```rust
#[derive(Props)]
struct ClickableProps<'a> {
    attributes: Attributes<'a>
}

fn clickable(cx: Scope<ClickableProps<'a>>) -> Element {
    cx.render(rsx!(
        a {
            ..cx.props.attributes,
            "Any link, anywhere"
        }
    ))
}
```

The quoted escapes are a great way to make your components more flexible.
 -->
