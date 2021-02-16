<div align="center">
  <h1>🌗🚀 Dioxus</h1>
  <p>
    <strong>A concurrent, functional, virtual DOM for Rust</strong>
  </p>
</div>

# About

Dioxus is a portable, performant, and ergonomic framework for building cross-platform user experiences in Rust.

```rust
static Example: FC<()> = |ctx| {
    let (value, set_value) = use_state(&ctx, || "...?");

    ctx.view(html! {
        <div>
            <button onclick={move |_| set_value("world!")}> "?" </button>
            <button onclick={move |_| set_value("Dioxus 🎉")}> "?" </button>
            <div>
                <h1> "Hello, {value}" </h1>
            </div>
        </div>
    })
};
```

Dioxus can be used to deliver webapps, desktop apps, static pages, liveview apps, Android apps, iOS Apps, and more. At its core, Dioxus is entirely renderer agnostic and has great documentation for creating new renderers for any platform.

Dioxus is supported by Dioxus Labs, a company providing end-to-end services for building, testing, deploying, and managing Dioxus apps on all supported platforms, designed especially for your next startup.

### Get Started with...
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



## Features
Dioxus' goal is to be the most advanced UI system for Rust, targeting isomorphism and hybrid approaches. Our goal is to eliminate context-switching for cross-platform development - both in UI patterns and programming language. Hooks and components should work *everywhere* without compromise.

Dioxus Core supports:
- [x] Hooks for component state
- [ ] Concurrent rendering
- [ ] Context subscriptions
- [ ] State management integrations

Separately, we maintain a collection of high quality, cross-platform hooks and services in the dioxus-hooks repo:
- [ ] `dioxus-router`: A hook-based router implementation for Dioxus web apps

We also maintain two state management options that integrate cleanly with Dioxus apps:
- [ ] `dioxus-reducer`: ReduxJs-style global state management
- [ ] `dioxus-dataflow`: RecoilJs-style global state management


## Explore
- **Running the examples**: Explore the vast collection of samples, tutorials, and demos
- **Building applications**: Use the Dioxus CLI to build and bundle apps for various platforms
- **Liveview**: Build custom liveview components that simplify datafetching on all platforms
- **State management**: Easily add powerful state management that comes integrated with Dioxus Core
- **Concurrency**: Drop in async where it fits and suspend components until new data is ready
- **1st party hooks**: router
- **Community hooks**: 3D renderers

## Dioxus LiveHost
Dioxus LiveHost is a paid service dedicated to hosting your Dioxus Apps - whether they be server-rendered, wasm-only, or a liveview. LiveHost enables a wide set of features:

- Versioned combined frontend and backend with unique access links
- Builtin CI/CD for all supported Dioxus platforms (macOS, Windows, Android, iOS, server, WASM, etc)
- Managed and pluggable storage database backends (PostgresSQL, Redis)
- Serverless support for minimal latency
- Analytics
- Lighthouse optimization
- On-premise support (see license terms)
- Cloudfare/DDoS protection integrations
- Web-based simulators for iOS, Android, Desktop
- Team + company management

For small teams, LiveHost is free 🎉. Check out the pricing page to see if Dioxus LiveHost is good fit for your team.


