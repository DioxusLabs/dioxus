# Introduction to Components

In the previous chapter, we learned about Elements and how they can be composed to create a basic User Interface. In this chapter, we'll learn how to group Elements together to form Components.

## What is a component?

In short, a component is a special function that takes an input and outputs a group of Elements. Typically, Components serve a single purpose: group functionality of a User Interface. Much like a function encapsulates some specific computation task, a Component encapsulates some specific rendering task.

Let's take the r/rust subreddit page and see if we can identify some parts of the page that are encapsulated as a single rendering task.


<!-- todo -->



Dioxus should look and feel just like writing functional React components. In Dioxus, there are no class components with lifecycles. All state management is done via hooks. This encourages logic reusability and lessens the burden on Dioxus to maintain a non-breaking lifecycle API.

```rust
#[derive(Properties, PartialEq)]
struct MyProps {
    name: String
}

fn Example(cx: Context<MyProps>) -> DomTree {
    html! { <div> "Hello {cx.props.name}!" </div> }
}
```

Here, the `Context` object is used to access hook state, create subscriptions, and interact with the built-in context API. Props, children, and component APIs are accessible via the `Context` object. The functional component macro makes life more productive by inlining props directly as function arguments, similar to how Rocket parses URIs.

The final output of components must be a tree of VNodes. We provide an html macro for using JSX-style syntax to write these, though, you could use any macro, DSL, templating engine, or the constructors directly.
