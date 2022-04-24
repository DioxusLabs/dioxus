# Managing State

Every app you'll build with Dioxus will have some sort of state that needs to be maintained and updated as your users interact with it. However, managing state can be particularly challenging at times, and is frequently the source of bugs in many GUI frameworks.

In this chapter, we'll cover the various ways to manage state, the appropriate terminology, various patterns, and some problems you might run into.


## The Problem

Why do people say state management is so difficult? What does it mean?

Generally, state management is the code you need to write to ensure that your app renders the *correct* content. If the user inputs a name, then you need to display the appropriate response - like alerts, validation, and disable/enable various elements on the page. Things can quickly become tricky if you need loading screens and cancellable tasks.

For the simplest of apps, all of your state can enter the app from the root props. This is common in server-side rendering - we can collect all of the required state *before* rendering the content.

```rust
let all_content = get_all_content().await;

let output = dioxus::ssr::render_lazy(rsx!{
    div {
        RenderContent { content: all_content }
    }
});
```

With this incredibly simple setup, it is highly unlikely that you'll have rendering bugs. There simply is barely any state to manage.

However, most of your apps will store state inside of the Dioxus VirtualDom - either through local state or global state.


## Your options

To deal with complexity, you have a couple of options:

- Refactor state out of shared state and into reusable components and hooks.
- Lift state upwards to be spread across multiple components (fan out).
- Use the Context API to share state globally.
- Use a dedicated state management solution like Fermi.
