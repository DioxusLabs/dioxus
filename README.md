<div align="center">
  <h1>🌗🚀 Dioxus</h1>
  <p>
    <strong>Frontend that scales.</strong>
  </p>
</div>

Dioxus is a portable, performant, and ergonomic framework for building cross-platform user experiences in Rust.

```rust
//! A complete dioxus web app
use dioxus_web::*;

struct Props { initial_text: &'static str }

fn Example(ctx: Context<Props>) -> VNode {
    let selection = use_state(ctx, move || ctx.initial_text);

    ctx.render(rsx! {
        div {
            h1 { "Hello, {selection}" }
            button { "?", onclick: move |_| selection.set("world!")}
            button { "?", onclick: move |_| selection.set("Dioxus 🎉")}
        }
    })
};

fn main() {
    dioxus_web::start_with_props(Example, Props { initial_text: "..?" }).block_on();
}
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

## FAQ

### Why?

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
- include_str! for integrating html/css/svg templates directly
- various macros (html!, rsx!) for fast template iteration

And much more. Dioxus makes Rust apps just as fast to write as React apps, but affords more robustness, giving your frontend team greater confidence in making big changes in shorter time. Dioxus also works on the server, on the web, on mobile, on desktop - and it runs completely natively so performance is never an issue.

### Immutability by default?

---

Rust, like JS and TS, supports both mutable and immutable data. With JS, `const` would be used to signify immutable data, while in rust, the absence of `mut` signifies immutable data.

Mutability:

```rust
let mut val = 10; // rust
let val = 10;     // js
```

Immutability

```rust
let val = 10;    // rust
const val = 10;  // js
```

However, `const` in JS does not prohibit you from modify the value itself only disallowing assignment. In Rust, immutable **is immutable**. You _never_ have to work about accidentally mutating data; mutating immutable data in Rust requires deliberate advanced datastructures that you won't find in your typical frontend code.

## How do strings work?

---

In rust, we have `&str`, `&'static str` `String`, and `Rc<str>`. It's a lot, yes, and it might be confusing at first. But it's actually not too bad.

In Rust, UTF-8 is supported natively, allowing for emoji and extended character sets (like Chinese and Arabic!) instead of the typical ASCII. The primitive `str` can be seen as a couple of UTF-8 code points squished together with a dynamic size. Because this size is variable (not known at compile time for any single character), we reference an array of UTF-8 code points as `&str`. Essentially, we're referencing (the & symbol) some dynamic `str` (a collection of UTF-8 points).

For text encoded directly in your code, this collection of UTF-8 code points is given the `'static` reference lifetime - essentially meaning the text can be safely referenced for the entire runtime of your program. Contrast this with JS, where a string will only exist for as long as code references it before it gets cleaned up by the garbage collector.

For text that needs to have characters added, removed, sorted, uppercased, formatted, accessed for mutation, etc, Rust has the `String` type, which is essentially just a dynamically sized `str`. In JS, if you add a character to your string, you actually create an entirely new string (completely cloning the old one first). In Rust, you can safely added characters to strings _without_ having to clone first, making string manipulation in Rust very efficient.

Finally, we have `Rc<str>`. This is essentially Rust's version of JavaScript's `string`. In JS, whenever you pass a `string` around (and don't mutate it), you don't actually clone it, but rather just increment a counter that says "this code is using this string." This counter prevents the garbage collector from deleting the string before your code is done using it. Only when all parts of your code are done with the string, will the string be deleted. `Rc<str>` works exactly the same way in Rust, but requires a deliberate `.clone()` to get the same behavior. In most instances, Dioxus will automatically do this for you, saving the trouble of having to `clone` when you pass an `Rc<str>` into child components. `Rc<str>` is typically better than `String` for Rust - it allows cheap sharing of strings, and through `make_mut` you can always produce your own mutable copy for modifying. You might not see `Rc<str>` in other Rust libraries as much, but you will see it in Dioxus due to Dioxus' aggressive memoization and focus on efficiency and performance.

If you run into issues with `&str`, `String`, `Rc<str>`, just try cloning and `to_string` first. For the vast majority of apps, the slight performance hit will be unnoticeable. Once you get better with Strings, it's very easy to go back and remove all the clones for more efficient alternatives, but you will likely never need to.
