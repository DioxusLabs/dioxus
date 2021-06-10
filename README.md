<div align="center">
  <h1>🌗🚀 Dioxus</h1>
  <p>
    <strong>Frontend that scales.</strong>
  </p>
</div>

Dioxus is a portable, performant, and ergonomic framework for building cross-platform user experiences in Rust.

```rust
fn Example(ctx: Context<()>) -> VNodes {
    let (selection, set_selection) = use_state(&ctx, move || "..?");

    ctx.render(rsx! {
        h1 { "Hello, {selection}" }
        button { "?", onclick: move |_| set_selection("world!")}
        button { "?", onclick: move |_| set_selection("Dioxus 🎉")}
    })
};
```

Dioxus can be used to deliver webapps, desktop apps, static pages, liveview apps, Android apps, iOS Apps, and more. At its core, Dioxus is entirely renderer agnostic and has great documentation for creating new renderers for any platform.

If you know React, then you already know Dioxus.

### **Things you'll love ❤️:**

- Ergonomic design
- Minimal boilerplate
- Familiar design and semantics
- Simple build, test, and deploy
- Support for html! and rsx! templating
- SSR, WASM, desktop, and mobile support
- Powerful and simple integrated state management
- Rust! (enums, static types, modules, efficiency)

## Get Started with...

<table style="width:100%" align="center">
    <tr >
        <th><a href="http://github.com/jkelleyrtp/dioxus">Web</a></th>
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

## Blog Posts

- [Why we need a stronger typed web]()
- [Isomorphic webapps in 10 minutes]()
- [Rust is high level too]()
- [Eliminating crashes with Rust webapps]()
- [Tailwind for Dioxus]()
- [The monoglot startup]()

## Why?

---

TypeScript is a great addition to JavaScript, but comes with a lot of tweaking flags, a slight performance hit, and an uneven ecosystem where some of the most important packages are not properly typed. TypeScript provides a lot of great benefits to JS projects, but comes with its own "tax" that can slow down dev teams. Rust can be seen as a step up from TypeScript, supporting:

- static types for _all_ libraries
- advanced pattern matching
- immutability by default
- clean, composable iterators
- a good module system
- integrated documentation
- inline built-in unit/integration testing
- best-in-class error handling
- simple and fast build system
- powerful standard library (no need for lodash or underscore)
- include_str! for integrating html/css/svg templates directly
- various macros (`html!`, `rsx!`) for fast template iteration

And much more. Dioxus makes Rust apps just as fast to write as React apps, but affords more robustness, giving your frontend team greater confidence in making big changes in shorter time. Dioxus also works on the server, on the web, on mobile, on desktop - and it runs completely natively so performance is never an issue.
