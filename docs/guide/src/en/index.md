# Introduction

![dioxuslogo](./images/dioxuslogo_full.png)

Dioxus is a portable, performant, and ergonomic framework for building cross-platform user interfaces in Rust. This guide will help you get started with writing Dioxus apps for the Web, Desktop, Mobile, and more.

```rust
fn app(cx: Scope) -> Element {
    let mut count = use_state(cx, || 0);

    cx.render(rsx!(
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    ))
}
```

Dioxus is heavily inspired by React. If you know React, getting started with Dioxus will be a breeze.

> This guide assumes you already know some [Rust](https://www.rust-lang.org/)! If not, we recommend reading [*the book*](https://doc.rust-lang.org/book/ch01-00-getting-started.html) to learn Rust first.

## Features

- Desktop apps running natively (no Electron!) in less than 10 lines of code.
- Incredibly ergonomic and powerful state management.
- Comprehensive inline documentation – hover and guides for all HTML elements, listeners, and events.
- Extremely memory efficient – 0 global allocations for steady-state components.
- Multi-channel asynchronous scheduler for first-class async support.
- And more! Read the [full release post](https://dioxuslabs.com/blog/introducing-dioxus/).

### Multiplatform

Dioxus is a *portable* toolkit, meaning the Core implementation can run anywhere with no platform-dependent linking. Unlike many other Rust frontend toolkits, Dioxus is not intrinsically linked to WebSys. In fact, every element and event listener can be swapped out at compile time. By default, Dioxus ships with the `html` feature enabled, but this can be disabled depending on your target renderer.

Right now, we have several 1st-party renderers:
- WebSys (for WASM): Great support
- Tao/Tokio (for Desktop apps): Good support
- Tao/Tokio (for Mobile apps): Poor support
- SSR (for generating static markup)
- TUI/Rink (for terminal-based apps): Experimental

## Stability

Dioxus has not reached a stable release yet.

Web: Since the web is a fairly mature platform, we expect there to be very little API churn for web-based features.

Desktop: APIs will likely be in flux as we figure out better patterns than our ElectronJS counterpart.

SSR: We don't expect the SSR API to change drastically in the future.
