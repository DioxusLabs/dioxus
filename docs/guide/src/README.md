# Introduction

![dioxuslogo](./images/dioxuslogo_full.png)

**Dioxus** is a library for building fast, scalable, and robust user interfaces with the Rust programming language. This guide will help you get started with Dioxus running on the Web, Desktop, Mobile, and more.

```rust
fn app(cx: Scope) -> Element {
    let (count, set_count) = use_state(&cx, || 0);

    cx.render(rsx!(
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| set_count(count + 1), "Up high!" }
        button { onclick: move |_| set_count(count - 1), "Down low!" }
    ))
};
```

In general, Dioxus and React share many functional similarities. If this guide is lacking in any general concept or an error message is confusing, React's documentation might be more helpful. We are dedicated to providing a *familiar* toolkit for UI in Rust, so we've chosen to follow in the footsteps of popular UI frameworks (React, Redux, etc). If you know React, then you already know Dioxus. If you don't know either, this guide will still help you!

> This is an introduction book! For advanced topics, check out the [Reference](/reference) instead.
