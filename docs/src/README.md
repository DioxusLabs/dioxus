# Introduction

![dioxuslogo](./images/dioxuslogo_full.png)

**Dioxus** is a framework and ecosystem for building fast, scalable, and robust user interfaces with the Rust programming language. This guide will help you get up-and-running with Dioxus running on the Web, Desktop, Mobile, and more. 

```rust, ignore
// An example Dioxus app - closely resembles React
static App: FC<()> = |(cx, props)| {
    let mut count = use_state(cx, || 0);

    cx.render(rsx!(
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    ))
};
```

The Dioxus API and patterns closely resemble React - if this guide is lacking in any general concept or an error message is confusing, we recommend substituting "React" for "Dioxus" in your web search terms. A major goal of Dioxus is to provide a familiar toolkit for UI in Rust, so we've chosen to follow in the footsteps of popular UI frameworks (React, Redux, etc) - if you know React, then you already know Dioxus. If you don't know either, this guide will still help you!



### Web Support
---

The Web is the most-supported target platform for Dioxus. To run on the Web, your app must be compiled to WebAssembly and depend on the `dioxus` crate with the `web` feature enabled. Because of the WASM limitation, not every crate will work with your web-apps, so you'll need to make sure that your crates work without native system calls (timers, IO, etc).

Because the web is a fairly mature platform, we expect there to be very little API churn for web-based features.

[Jump to the getting started guide for the web.]()

Examples:
- [TodoMVC](https://github.com/dioxusLabs/todomvc/)
- [ECommerce]()
- [Photo Editor]()

[![todomvc](https://github.com/DioxusLabs/todomvc/raw/master/example.png)](https://github.com/dioxusLabs/todomvc/)

### SSR Support
---
Dioxus supports server-side rendering! In a pinch, you can literally "debug" the VirtualDom:

```rust
let dom = VirtualDom::new(App);
println!("{:?}, dom");
```

For rendering statically to an `.html` file or from a WebServer, then you'll want to make sure the `ssr` feature is enabled in the `dioxus` crate and use the `dioxus::ssr` API. We don't expect the SSR API to change drastically in the future.

```rust
let contents = dioxus::ssr::render_vdom(&dom, |c| c);
```


[Jump to the getting started guide for SSR.]()

Examples:
- [Example DocSite]()
- [Tide WebServer]()
- [Markdown to fancy HTML generator]()

### Desktop Support
---
The desktop is a powerful target for Dioxus, but is currently limited in capability when compared to the Web platform. Currently, desktop apps are rendered with the platform's WebView library, but your Rust code is running natively on a native thread. This means that browser APIs are *not* available, so rendering WebGL, Canvas, etc is not as easy as the Web. However, native system APIs *are* accessible, so streaming, WebSockets, filesystem, etc are all viable APIs. In the future, we plan to move to a custom webrenderer-based DOM renderer with WGPU integrations.

Desktop APIs will likely be in flux as we figure out better patterns than our ElectronJS counterpart.

[Jump to the getting started guide for Desktop.]()

Examples:
- [File explorer]()
- [Bluetooth scanner]()
- [Device Viewer]()

[![FileExplorerExample](https://github.com/DioxusLabs/file-explorer-example/raw/master/image.png)](https://github.com/dioxusLabs/file-explorer/)

### Mobile Support
---
Mobile is currently the least-supported renderer target for Dioxus. Mobile apps are rendered with the platform's WebView, meaning that animations, transparency, and native widgets are not currently achievable. In addition, iOS is the only supported Mobile Platform. It is possible to get Dioxus running on Android and rendered with WebView, but the Rust windowing library that Dioxus uses - tao - does not currently supported Android.

[Jump to the getting started guide for Mobile.]()

Examples:
- [Todo App]()
- [Chat App]()

### LiveView Support
---

The internal architecture of Dioxus was designed from day one to support the `LiveView` use-case, where a web server hosts a running app for each connected user. As of today, there is no out-of-the-box LiveView support - you'll need to wire this up yourself. While not currently fully implemented, the expectation is that LiveView apps can be a hybrid between WASM and server-rendered where only portions of a page are "live" and the rest of the page is either server-rendered, statically generated, or handled by the host SPA.



### Multithreaded Support
---
The Dioxus VirtualDom, sadly, is not `Send.` This means you can't easily use Dioxus with most web frameworks like Tide, Rocket, Axum, etc. Currently, your two options include:
- Actix: Actix supports `!Send` handlers
- VirtualDomPool: A thread-per-core VirtualDom pool that uses message passing and serialization

When working with web frameworks that require `Send`, it is possible to render a VirtualDom immediately to a String - but you cannot hold the VirtualDom across an await point. For retained-state SSR (essentially LiveView), you'll need to create a VirtualDomPool which solves the `Send` problem.
