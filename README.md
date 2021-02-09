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
            <button onclick={move |_| set_value("world!")}> "world" </button>
            <button onclick={move |_| set_value("dioxus 🎉")}> "dioxus" </button>
            <div>
                <p> "Hello, {val1}" </p>
            </div>
        </div>
    })
};
```
Dioxus can be used to serve webapps, desktop apps, static pages, LiveView apps, Android apps, iOS Apps, and more. At its core,
Dioxus is entirely renderer agnostic and has great documentation for creating new renderers for any platform.

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



## Components
Dioxus should look and feel just like writing functional React components. In Dioxus, there are no class components with lifecycles. All state management is done via hooks. This encourages logic reusability and lessens the burden on Dioxus to maintain a non-breaking lifecycle API.

```rust
#[derive(Properties, PartialEq)]
struct MyProps {
    name: String
}

fn Example(ctx: Context<MyProps>) -> VNode {
    html! { <div> "Hello {ctx.props.name}!" </div> }
}
```

Here, the `Context` object is used to access hook state, create subscriptions, and interact with the built-in context API. Props, children, and component APIs are accessible via the `Context` object. The functional component macro makes life more productive by inlining props directly as function arguments, similar to how Rocket parses URIs.

```rust
// A very terse component!
#[fc]
fn Example(ctx: Context, name: String) -> VNode {
    html! { <div> "Hello {name}!" </div> }
}

// or

#[functional_component]
static Example: FC = |ctx, name: String| html! { <div> "Hello {name}!" </div> }; 
```

The final output of components must be a tree of VNodes. We provide an html macro for using JSX-style syntax to write these, though, you could use any macro, DSL, templating engine, or the constructors directly. 

## Concurrency
In Dioxus, VNodes are asynchronous and can their rendering can be paused at any time by awaiting a future. Hooks can combine this functionality with the Context and Subscription APIs to craft dynamic and efficient user experiences. 

```rust
fn user_data(ctx: Context<()>) -> VNode {
    // Register this future as a task
    use_suspense(ctx, async {
        // Continue on with the component as usual, waiting for data to arrive
        let Profile { name, birthday, .. } = fetch_data().await;
        html! {
            <div>
                {"Hello, {name}!"}
                {if birthday === std::Instant::now() {html! {"Happy birthday!"}}}
            </div>
        }
    })
}
```
Asynchronous components are powerful but can also be easy to misuse as they pause rendering for the component and its children. Refer to the concurrent guide for information on how to best use async components. 

## Liveview
With the Context, Subscription, and Asynchronous APIs, we've built Dioxus Liveview: a coupling of frontend and backend to deliver user experiences that do not require dedicated API development. Instead of building and maintaining frontend-specific API endpoints, components can directly access databases, server caches, and other services directly from the component.

These set of features are still experimental. Currently, we're still working on making these components more ergonomic

```rust
fn live_component(ctx: &Context<()>) -> VNode {
    use_live_component(
        ctx,
        // Rendered via the client
        #[cfg(target_arch = "wasm32")]
        || html! { <div> {"Loading data from server..."} </div> },

        // Renderered on the server
        #[cfg(not(target_arch = "wasm32"))]
        || html! { <div> {"Server Data Loaded!"} </div> },
    )
}
```

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

## Examples
We use the dedicated `dioxus-cli` to build and test dioxus web-apps. This can run examples, tests, build web workers, launch development servers, bundle, and more. It's general purpose, but currently very tailored to Dioxus for liveview and bundling. If you've not used it before, `cargo install --path pacakages/dioxus-cli` will get it installed. This CLI tool should feel like using `cargo` but with 1st party support for assets, bundling, and other important dioxus-specific features.

Alternatively, `trunk` works but can't run examples.

- tide_ssr: Handle an HTTP request and return an html body using the html! macro. `cargo run --example tide_ssr`
- doc_generator: Use dioxus SSR to generate the website and docs. `cargo run --example doc_generator`
- fc_macro: Use the functional component macro to build terse components. `cargo run --example fc_macro`
- hello_web: Start a simple wasm app. Requires a web packer like dioxus-cli or trunk `cargo run --example hello`
- router: `cargo run --example router`
- tide_ssr: `cargo run --example tide_ssr`
- webview: Use liveview to bridge into a webview context for a simple desktop application. `cargo run --example webview`
- twitter-clone: A full-featured Twitter clone showcasing dioxus-liveview, state management patterns, and hooks. `cargo run --example twitter`

## Documentation


