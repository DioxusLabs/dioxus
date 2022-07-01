# Introduction

![dioxuslogo](./images/dioxuslogo_full.png)

Dioxus is a Rust framework for building fast, scalable, and robust user interfaces. This guide will help you get started with writing Dioxus apps for the Web, Desktop, Mobile, and more.

```rust
fn app(cx: Scope) -> Element {
    let mut count = use_state(&cx, || 0);

    cx.render(rsx!(
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    ))
};
```

Dioxus is heavily inspired by React. If you know React, getting started with Dioxus will be a breeze.

> This guide assumes you already know some [Rust](https://www.rust-lang.org/)! If not, we recommend reading [*the book*](https://doc.rust-lang.org/book/ch01-00-getting-started.html) to learn Rust first.

## Multiplatform

Dioxus is a *portable* toolkit, meaning the Core implementation can run anywhere with no platform-dependent linking. Unlike many other Rust frontend toolkits, Dioxus is not intrinsically linked to WebSys. In fact, every element and event listener can be swapped out at compile time. By default, Dioxus ships with the `html` feature enabled, but this can be disabled depending on your target renderer.

Right now, we have several 1st-party renderers:
- WebSys (for WASM): Great support
- Tao/Tokio (for Desktop apps): Good support
- Tao/Tokio (for Mobile apps): Poor support
- SSR (for generating static markup)
- TUI/Rink (for terminal-based apps): Experimental

### LiveView / Server Component Support

The internal architecture of Dioxus was designed from day one to support the `LiveView` use-case, where a web server hosts a running app for each connected user. As of today, there is no first-class LiveView support - you'll need to wire this up yourself.

While not currently fully implemented, the expectation is that LiveView apps can be a hybrid between Wasm and server-rendered where only portions of a page are "live" and the rest of the page is either server-rendered, statically generated, or handled by the host SPA.

### Multithreaded Support
The Dioxus VirtualDom, sadly, is not currently `Send`. Internally, we use quite a bit of interior mutability which is not thread-safe. This means you can't easily use Dioxus with most web frameworks like Tide, Rocket, Axum, etc.

To solve this, you'll want to spawn a VirtualDom on its own thread and communicate with it via channels.

When working with web frameworks that require `Send`, it is possible to render a VirtualDom immediately to a String - but you cannot hold the VirtualDom across an await point. For retained-state SSR (essentially LiveView), you'll need to create a pool of VirtualDoms.

Ultimately, you can always wrap the VirtualDom with a `Send` type and manually uphold the `Send` guarantees yourself.


## Stability

Dioxus has not reached a stable release yet.

Web: Since the web is a fairly mature platform, we expect there to be very little API churn for web-based features.

Desktop: APIs will likely be in flux as we figure out better patterns than our ElectronJS counterpart.

SSR: We don't expect the SSR API to change drastically in the future.