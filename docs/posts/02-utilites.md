# Utilities

There are a few macros and utility functions that make life easier when writing Dioxus components.

## The `functional_component` procedural macro

The `functional_component` proc macro allows you to inline props into the generic parameter of the component's context. This is useful When writing "pure" components, or when you don't want the extra clutter of structs, derives, and burden of naming things.

This macro allows allows a classic struct definition to be embedded directly into the function arguments. The props are automatically pulled from the context and destructured into the function's body, saving an extra step.

```rust
// Inlines and destructure props *automatically*
#[functional_component]
fn Example(cx: Context, name: &str, pending: bool, count: i32 ) -> VNode {
    html! {
        <div>
            <p> "Hello, {name}!" </p>
            <p> "Status: {pending}!" </p>
            <p> "Count {count}!" </p>
        </div>
    }
}
```

becomes this:

```rust
#[derive(Debug, Properties, PartialEq)]
struct ExampleProps {
     name: String
     pending: bool
     count: i32
};

fn Example(cx: &mut Context<ExampleProps>) -> VNode {
    let ExampleProps {
        name, pending, count
    } = cx.props;

    rsx! {
        <div>
            <p> "Hello, {name}!" </p>
            <p> "Status: {pending}!" </p>
            <p> "Count {count}!" </p>
        </div>
    }
}
```

## The rsx! macro

The rsx! macacro is similar to the html! macro in other libraries, but with a few add-ons to make it fun and easy to work with. We'll cover the rsx macro more in depth in the [vnode-macro](3-vnode-macros.md) chapter.
