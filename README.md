<div align="center">
  <h1>🌗🚀 Dioxus</h1>
  <p>
    <strong>Frontend that scales.</strong>
  </p>
</div>

<!-- # About -->

Dioxus is a portable, performant, and ergonomic framework for building cross-platform user experiences in Rust.


```rust
#[fc]
static Example: FC<()> = |ctx, props| {
    let (selection, set_selection) = use_state(&ctx, || "...?");

    ctx.render(rsx! {
        div {
            h1 { "Hello, {selection}" }
            button { "?", onclick: move |_| set_selection("world!")}
            button { "?", onclick: move |_| set_selection("Dioxus 🎉")}
        }
    })
};
```

Dioxus can be used to deliver webapps, desktop apps, static pages, liveview apps, Android apps, iOS Apps, and more. At its core, Dioxus is entirely renderer agnostic and has great documentation for creating new renderers for any platform.

### **Things you'll love ❤️:**
- Ergonomic design
- Minimal boilerplate
- Simple build, test, and deploy
- Support for html! and rsx! templating
- SSR, WASM, desktop, and mobile support
- Rust! (enums, static types, modules, efficiency)
  

## Get Started with...
<table style="width:100%" align="center">
    <tr >
        <th><a href="http://github.com/jkelleyrtp/dioxus">WebApps</a></th>
        <th><a href="http://github.com/jkelleyrtp/dioxus">Desktop</a></th>
        <th><a href="http://github.com/jkelleyrtp/dioxus">Mobile</a></th>
        <th><a href="http://github.com/jkelleyrtp/dioxus">State Management</a></th>
        <th><a href="http://github.com/jkelleyrtp/dioxus">Docs</a></th>
        <th><a href="http://github.com/jkelleyrtp/dioxus">Tools</a></th>
    <tr>
</table>





## Explore
- [**HTML Templates**: Drop in existing HTML5 templates with html! macro](docs/guides/00-index.md)
- [**RSX Templates**: Clean component design with rsx! macro](docs/guides/00-index.md)
- [**Running the examples**: Explore the vast collection of samples, tutorials, and demos](docs/guides/00-index.md)
- [**Building applications**: Use the Dioxus CLI to build and bundle apps for various platforms](docs/guides/01-ssr.md)
- [**Liveview**: Build custom liveview components that simplify datafetching on all platforms](docs/guides/01-ssr.md)
- [**State management**: Easily add powerful state management that comes integrated with Dioxus Core](docs/guides/01-ssr.md)
- [**Concurrency**: Drop in async where it fits and suspend components until new data is ready](docs/guides/01-ssr.md)
- [**1st party hooks**: Cross-platform router hook](docs/guides/01-ssr.md)
- [**Community hooks**: 3D renderers](docs/guides/01-ssr.md)


---
## Dioxus LiveHost
Dioxus LiveHost is a paid service that accelerates the deployment of Dioxus Apps. It provides CI/CD, testing, monitoring, scaling, and deployment specifically for Dioxus apps. 
- It's the fastest way of launching your next internal tool, side-project, or startup. 🚀


<!-- Dioxus LiveHost is a paid service dedicated to hosting your Dioxus Apps - whether they be server-rendered, wasm-only, or a liveview. It's  -->

<!-- LiveHost enables a wide set of features: -->
<!-- 
- Versioned combined frontend and backend with unique access links
- Builtin CI/CD for all supported Dioxus platforms (macOS, Windows, Android, iOS, server, WASM, etc)
- Managed and pluggable storage database backends (PostgresSQL, Redis)
- Serverless support for minimal latency
- Analytics
- Lighthouse optimization
- On-premise support (see license terms)
- Cloudfare/DDoS protection integrations
- Web-based simulators for iOS, Android, Desktop
- Team + company management -->

<!-- For small teams, LiveHost is free 🎉. Check out the pricing page to see if Dioxus LiveHost is good fit for your team. -->
