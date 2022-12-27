# Sharing State

Often, multiple components need to access the same state. Depending on your needs, there are several ways to implement this.

## Lifting State

One approach to share state between components is to "lift" it up to the nearest common ancestor. This means putting the `use_state` hook in a parent component, and passing the needed values down as props.

Suppose we want to build a meme editor. We want to have an input to edit the meme caption, but also a preview of the meme with the caption. Logically, the meme and the input are 2 separate components, but they need access to the same state (the current caption).

> Of course, in this simple example, we could write everything in one component – but it is better to split everything out in smaller components to make the code more reusable, maintainable, and performant (this is even more important for larger, complex apps).

We start with a `Meme` component, responsible for rendering a meme with a given caption:
```rust
{{#include ../../../examples/meme_editor.rs:meme_component}}
```

> Note that the `Meme` component is unaware where the caption is coming from – it could be stored in `use_state`, `use_ref`, or a constant. This ensures that it is very reusable – the same component can be used for a meme gallery without any changes!

We also create a caption editor, completely decoupled from the meme. The caption editor must not store the caption itself – otherwise, how will we provide it to the `Meme` component? Instead, it should accept the current caption as a prop, as well as an event handler to delegate input events to:

```rust
{{#include ../../../examples/meme_editor.rs:caption_editor}}
```

Finally, a third component will render the other two as children. It will be responsible for keeping the state and passing down the relevant props.
```rust
{{#include ../../../examples/meme_editor.rs:meme_editor}}
```
![Meme Editor Screenshot: An old plastic skeleton sitting on a park bench. Caption: "me waiting for a language feature"](./images/meme_editor_screenshot.png)

## Using Context

Sometimes, some state needs to be shared between multiple components far down the tree, and passing it down through props is very inconvenient.

Suppose now that we want to implement a dark mode toggle for our app. To achieve this, we will make every component select styling depending on whether dark mode is enabled or not.

> Note: we're choosing this approach for the sake of an example. There are better ways to implement dark mode (e.g. using CSS variables). Let's pretend CSS variables don't exist – welcome to 2013!

Now, we could write another `use_state` in the top component, and pass `is_dark_mode` down to every component through props. But think about what will happen as the app grows in complexity – almost every component that renders any CSS is going to need to know if dark mode is enabled or not – so they'll all need the same dark mode prop. And every parent component will need to pass it down to them. Imagine how messy and verbose that would get, especially if we had components several levels deep!

Dioxus offers a better solution than this "prop drilling" – providing context. The [`use_context_provider`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_context_provider.html) hook is similar to `use_ref`, but it makes it available through [`use_context`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_context.html) for all children components.

First, we have to create a struct for our dark mode configuration:

```rust
{{#include ../../../examples/meme_editor_dark_mode.rs:DarkMode_struct}}
```

Now, in a top-level component (like `App`), we can provide the `DarkMode` context to all children components:
```rust
{{#include ../../../examples/meme_editor_dark_mode.rs:context_provider}}
```

As a result, any child component of `App` (direct or not), can access the `DarkMode` context.
```rust
{{#include ../../../examples/meme_editor_dark_mode.rs:use_context}}
```

> `use_context` returns `Option<UseSharedState<DarkMode>>` here. If the context has been provided, the value is `Some(UseSharedState<DarkMode>)`, which you can call `.read` or `.write` on, similarly to `UseRef`. Otherwise, the value is `None`.

For example, here's how we would implement the dark mode toggle, which both reads the context (to determine what color it should render) and writes to it (to toggle dark mode):
```rust
{{#include ../../../examples/meme_editor_dark_mode.rs:toggle}}
```

