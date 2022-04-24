# Introducing Dioxus v0.1 ✨

> Jan 3, 2022

> [@jkelleyrtp](https://github.com/jkelleyrtp), thanks [@alexkirsz](https://github.com/alexkirsz)

After many months of work, we're very excited to release the first version of Dioxus!

Dioxus is a new library for building interactive user interfaces (GUI) with Rust. It is built around a Virtual DOM, making it portable for the web, desktop, server, mobile, and more. 

Dioxus has the following design goals:

- **Familiar**: Offer a React-like mental model and API surface
- **Robust**: Avoid runtime bugs by moving rules and error handling into the type system
- **Performant**: Scale to the largest apps and the largest teams
- **Productive**: Comprehensive inline documentation, fast recompiles, and deeply integrated tooling
- **Extensible**: Reusable hooks and components that work on every platform

Dioxus is designed to be familiar for developers already comfortable with React paradigms. Our goal is to ensure a smooth transition from TypeScript/React without having to learn any major new concepts.

To give you an idea of what Dioxus looks like, here's a simple counter app:

```rust
use dioxus::prelude::*;

fn main() {
	dioxus::desktop::launch(app)
}

fn app(cx: Scope) -> Element {
    let mut count = use_state(&cx, || 0);

    cx.render(rsx! {
        h1 { "Count: {count}" }
        button { onclick: move |_| count += 1, "+" }
        button { onclick: move |_| count -= 1, "-" }
    })
}
```

This simple counter is a complete desktop application, running at native speeds on a native thread. Dioxus automatically shuttles all events from the WebView runtime into the application code. In our app, we can interact natively with system APIs, run multi-threaded code, and do anything a regular native Rust application might do. Running `cargo build --release` will compile a portable binary that looks and feels the same on Windows, macOS, and Linux. We can then use `cargo-bundle` to bundle our binary into a native `.app`/`.exe`/`.deb`.

Dioxus supports many of the same features React does including:

- Server-side-rendering, pre-rendering, and hydration
- Mobile, desktop, and web support
- Suspense, fibers, coroutines, and error handling
- Hooks, first-class state management, components
- Fragments, conditional rendering, and custom elements

However, some things are different in Dioxus:

- Automatic memoization (opt-out rather than opt-in)
- No effects - effectual code can only originate from actions or coroutines
- Suspense is implemented as hooks - _not_ deeply ingrained within Dioxus Core
- Async code is _explicit_ with a preference for _coroutines_ instead

As a demo, here's our teaser example running on all our current supported platforms:

![Teaser Example](/static/Untitled.png)

This very site is built with Dioxus, and the source code is available [here](https://github.com/dioxuslabs/docsite).

To get started with Dioxus, check out any of the "Getting Started" guides for your platform of choice, or check out the GitHub Repository for more details.

- [Getting Started with Dioxus](https://dioxuslabs.com/guide)
- [Getting Started with Web](https://dioxuslabs.com/reference/web)
- [Getting Started with Desktop](https://dioxuslabs.com/reference/desktop)
- [Getting Started with Mobile](https://dioxuslabs.com/reference/mobile)
- [Getting Started with SSR](https://dioxuslabs.com/reference/ssr)

## Show me some examples of what can be built!

- [File explorer desktop app](https://github.com/dioxuslabs/example-projects)
- [WiFi scanner desktop app](https://github.com/dioxuslabs/example-projects)
- [Dog CEO API Search](https://github.com/dioxuslabs/example-projects)
- [TodoMVC Mobile App](https://github.com/dioxuslabs/example-projects)
- [E-Commerce Liveview App](https://github.com/dioxuslabs/example-projects)

## Why should I use Rust and Dioxus for frontend?

We believe that Rust's ability to write high-level and statically typed code should make it easier for frontend teams to take on even the most ambitious of projects. Rust projects can be refactored fearlessly: the powerful type system prevents an entire class of bugs at compile-time. No more `cannot read property of undefined` ever again! With Rust, all errors must be accounted for at compile time. You cannot ship an app that does not — in some way — handle its errors.

### Difference from TypeScript/React:

TypeScript is still fundamentally JavaScript. If you've written enough TypeScript, you might be bogged down with lots of configuration options, lack of proper support for "go-to-source," or incorrect ad-hoc typing. With Rust, strong types are built-in, saving tons of headache like `cannot read property of undefined`.

By using Rust, we gain:

- Strong types for every library
- Immutability by default
- A simple and intuitive module system
- Integrated documentation (go to source actually goes to source instead of the `.d.ts` file)
- Advanced pattern matching
- Clean, efficient, composable iterators
- Inline built-in unit/integration testing
- High quality error handling
- Flexible standard library and traits
- Powerful macro system
- Access to the [crates.io](https://crates.io) ecosystem

Dioxus itself leverages this platform to provide the following guarantees:

- Correct use of immutable data structures
- Guaranteed handling of errors and null-values in components
- Native performance on mobile
- Direct access to system IO

And much more. Dioxus makes Rust apps just as fast to write as React apps, but affords more robustness, giving your frontend team greater confidence in making big changes in shorter timespans.

Semantically, TypeScript-React and Rust-Dioxus are very similar. In TypeScript, we would declare a simple component as:

```tsx
type CardProps = {
  title: string,
  paragraph: string,
};

const Card: FunctionComponent<CardProps> = (props) => {
  let [count, set_count] = use_state(0);
  return (
    <aside>
      <h2>{props.title}</h2>
      <p> {props.paragraph} </p>
	  <button onclick={() => set_count(count + 1)}> Count {count} </button>
    </aside>
  );
};
```

In Dioxus, we would define the same component in a similar fashion:

```rust
#[derive(Props, PartialEq)]
struct CardProps {
	title: String,
	paragraph: String
}

static Card: Component<CardProps> = |cx| {
	let mut count = use_state(&cx, || 0);
	cx.render(rsx!(
		aside {
			h2 { "{cx.props.title}" }
			p { "{cx.props.paragraph}" }
			button { onclick: move |_| count+=1, "Count: {count}" }
		}
	))
};
```

However, we recognize that not every project needs Rust - many are fine with JavaScript! We also acknowledge that Rust/Wasm/Dioxus does not fix "everything that is wrong with frontend development." There are always going to be new patterns, frameworks, and languages that solve these problems better than Rust and Dioxus.

As a general rule of thumb, Dioxus is for you if:

- your app will become very large
- you need to share code across many platforms
- you want a fast way to build for desktop
- you want to avoid electron or need direct access to hardware
- you're tired of JavaScript tooling

Today, to publish a Dioxus app, you don't need NPM/WebPack/Parcel/etc. Dioxus simply builds with cargo, and for web builds, Dioxus happily works with the popular [trunk](http://trunkrs.dev) project.

## Show me more

Here, we'll dive into some features of Dioxus and why it's so fun to use. The [guide](https://dioxuslabs.com/guide/) serves as a deeper and more comprehensive look at what Dioxus can do.

## Building a new project is simple

To start a new project, all you need is Cargo, which comes with Rust. For a simple desktop app, all we'll need is the `dioxus` crate with the appropriate `desktop` feature. We start by initializing a new binary crate:

```shell
$ cargo init dioxus_example
$ cd dioxus_example
```

We then add a dependency on Dioxus to the `Cargo.toml` file, with the "desktop" feature enabled:

```rust
[dependencies]
dioxus = { version = "*", features = ["desktop"] }
```

We can add our counter from above.

```rust
use dioxus::prelude::*;

fn main() {
	dioxus::desktop::launch(app)
}

fn app(cx: Scope) -> Element {
    let mut count = use_state(&cx, || 0);

    cx.render(rsx! {
        h1 { "Count: {count}" }
        button { onclick: move |_| count += 1, "+" }
        button { onclick: move |_| count -= 1, "-" }
    })
}
```

And voilà! We can `cargo run` our app


![Simple Counter Desktop App](/static/counter.png)

## Support for JSX-style templating

Dioxus ships with a templating macro called RSX, a spin on React's JSX. RSX is very similar to regular struct syntax for Rust so it integrates well with your IDE. If used with [Rust-Analyzer](https://github.com/rust-analyzer/rust-analyzer) (not tested anywhere else) RSX supports code-folding, block selection, bracket pair colorizing, autocompletion, symbol renaming — pretty much anything you would expect from writing regular struct-style code.

```rust
rsx! {
	div { "Hello world" }
	button {
		onclick: move |_| log::info!("button pressed"),
		"Press me"
	}
}
```

If macros aren't your style, you can always drop down to the factory API:

```rust
LazyNodes::new(|f| {
	f.fragment([
		f.element(div, [f.text("hello world")], [], None, None)
		f.element(
			button,
			[f.text("Press Me")],
			[on::click(move |_| log::info!("button pressed"))],
			None,
			None
		)
	])
})
```

The `rsx!` macro generates idiomatic Rust code that uses the factory API — no different than what you'd write by hand yourself.

To make it easier to work with RSX, we've built a small [VSCode extension](https://github.com/DioxusLabs/studio) with useful utilities. This extension provides a command that converts a selected block of HTML into RSX so you can easily reuse existing web templates. 

## Dioxus prioritizes developer experience

Many of the Rust UI frameworks are particularly difficult to work with. Even the ones branded as "ergonomic" are quite challenging to in comparison to TSX/JSX. With Dioxus, we've innovated on a number of Rust patterns to deliver a framework that is actually enjoyable to develop in.

For example, many Rust frameworks require you to clone your data in for *every* closure and handler you use. This can get really clumsy for large apps.

```rust
div()
	.children([
		button().onclick(cloned!(name, date, age, description => move |evt| { /* */ })
		button().onclick(cloned!(name, date, age, description => move |evt| { /* */ })
		button().onclick(cloned!(name, date, age, description => move |evt| { /* */ })
	])
```

Dioxus understands the lifetimes of data borrowed from `Scope`, so you can safely return any borrowed data without declaring explicit captures. Hook handles all implement `Copy` so they can be shared between listeners without any ceremony.


```rust
let name = use_state(&cx, || "asd");
rsx! {
	div {
		button { onclick: move |_| name.set("abc") }
		button { onclick: move |_| name.set("def") }
		button { onclick: move |_| name.set("ghi") }
	}
}
```

Because we know the lifetime of your handlers, we can also expose this to children. No other Rust frameworks let us share borrowed state through the tree, forcing use of Rc/Arc everywhere. With Dioxus, all the Rc/Arc magic is tucked away in hooks, and just beautiful borrowed interfaces are exposed to your code. You don't need to know how Rc/RefCell work to build a competent Dioxus app.

```rust
fn app(cx: Scope) -> Element {
	let name = use_state(&cx, || "asd");
	cx.render(rsx!{
		Button { name: name }
	})
}

#[derive(Props)]
struct ButtonProps<'a> {
	name: UseState<'a, &'static str>
}

fn Button<'a>(cx: Scope<'a, Childprops<'a>>) -> Element {
	cx.render(rsx!{
		button {
			onclick: move |_| cx.props.name.set("bob")
		}
	})
}
```

There's *way* more to this story, but hopefully we've convinced you that Dioxus' DX somewhat approximates JSX/React.


## Dioxus is perfected for the IDE

Note: all IDE-related features have only been tested with [Rust-Analyzer](https://github.com/rust-analyzer/rust-analyzer). 

Dioxus code operates pleasantly with your IDE. For starters, most elements are documented through the Rustdoc system. A quick summary of the MDN docs is always under your finger tips:

![Elements have hover context](/static/ide_hover.png)

Dioxus also wraps platform-specific events with a custom synthetic event system. This means events enjoy proper autocomplete and documentation, unlike [Yew](https://yew.rs/) which currently relies on [web-sys](https://crates.io/crates/web-sys) with incomplete IDE support:

![Events are strongly typed](/static/ide_autocomplete.png)

Even element attributes and event handlers have top-notch documentation!

![Element attributes and listeners have hover context](/static/ide_listener.png)

The `rsx!` macro also benefits from code folding, batch renaming, and block selection, making most basic code navigation and completion tasks a breeze.

![Element blocks can be folded and renamed](/static/ide_selection.png)

Furthermore, the `rsx!` macro itself is documented, so if you ever forget how to use a certain feature, the documentation remains close at hand:

![The RSX documentation is provided on hover](/static/ide_rsx.png)

We spent a ton of time on this and we hope you enjoy it!

## Dioxus is extremely fast

We take the performance of Dioxus seriously. Instead of resolving to "good enough," Dioxus is designed to push the limits of what a declarative React-like framework can achieve. Dioxus is designed with multi-tenancy in mind: a single machine should be able to run thousands of simultaneous low-latency LiveView apps without skipping a beat. To accomplish this goal we've implemented a large number of optimizations:

- Usage of bump memory allocators and double-buffering
- Compile-time hashing of templates
- Automatic component memoization
- Fiber-like scheduler
- DOM Patch Batching

Dioxus is humbly built off the work done by [Dodrio](https://github.com/fitzgen/dodrio), a now-archived research project by fitzgen exploring the use of bump allocators in UI frameworks.

Dioxus is *substantially* more performant than many of the other Rust DOM-based UI libraries (Yew/Percy) and is *significantly* more performant than React - roughly competitive with InfernoJS. While not as performant as libraries like SolidJS/Sycamore, Dioxus imposes roughly a ~3% overhead over DOM patching, so it's *plenty* fast.

## Works on Desktop and Mobile 
We’ve mentioned before that Dioxus works practically anywhere that Rust does. When running natively as a desktop or mobile app, your Dioxus code will run on its own thread, not inside of a web runtime. This means you can access hardware, file system, and platform APIs directly without needing to go through a shim layer. In our examples, we feature a [file explorer app](https://github.com/DioxusLabs/example-projects/tree/master/file-explorer) and [WiFi scanner app](https://github.com/DioxusLabs/example-projects/tree/master/wifi-scanner) where platform access occurs inside an asynchronous multithreaded coroutine. This solves the problem faced by React Native and other cross-platform toolkits where JavaScript apps incur a massive performance penalty with substantial maintenance overhead associated with platform API shims.

A desktop app:

[![Example Dioxus desktop app](https://github.com/DioxusLabs/example-projects/raw/master/file-explorer/image.png)](https://github.com/DioxusLabs/example-projects/blob/master/file-explorer)

A mobile app:

[![Example Dioxus mobile app](https://github.com/DioxusLabs/example-projects/raw/master/ios_demo/assets/screenshot_smaller.jpeg)](https://github.com/DioxusLabs/example-projects/blob/master/ios_demo)

However, be warned that mobile is currently considered very experimental and there will likely be quirks. Dioxus is leveraging the work done by the [Tauri](https://github.com/tauri-apps/tauri) team to enable mobile support, and mobile support isn't technically complete in Tauri yet.

iOS should be supported out of the box, but Android support will take custom some boilerplate that hasn't been completely figured out. If you're interested in contributing to Dioxus, improving mobile support would be extremely helpful.

### Did someone say TUI support?

Yes, you can even build terminal user interfaces with Dioxus. Full support is still a work in progress, but the foundation is there.

[![TUI Support](https://github.com/DioxusLabs/rink/raw/master/examples/example.png)](https://github.com/dioxusLabs/rink)

### Things we didn't cover:

There are a bunch of things we didn't talk about here. Check out the guide for more information, or peruse the examples and reference for more context.

- Jank-free rendering with fiber scheduler
- [Support for borrowed props]()
- [Conditional rendering]()
- [CSS/Styling/Inline style support]()
- [Support for inline Context Providing/Consuming]()
- [First-class global state management]()

For a quick glance at party with React, check out the [Readme on Github](https://github.com/DioxusLabs/dioxus#parity-with-react).

## What's on the roadmap?

The world of Rust on the frontend is barely explored. Given the performance, ergonomics, and portability of Rust/Dioxus, we expect there to be a ton of different applications where having a React-like toolkit running natively can enable things previously considered impossible.

In the coming weeks, our plan is to finish the remaining outstanding features where Dioxus is lacking in comparison to React:

- Transition effects for Suspense
- Micro-optimizations and better cross-platform/browser bug mitigations
- Heuristics to guide the diffing algorithm
- Better support for subtree memoization (signals, etc.)
- More thorough documentation, fleshing out sore spots

We also need some help in important crates currently missing:

- First class cross-platform router (currently in progress)
- An extension to DioxusStudio that enables lazy bundling of static assets
- Animation library (see [React Spring](https://react-spring.io/), [Framer Motion](https://www.framer.com/motion/))
- A [TUI renderer for Dioxus](https://github.com/dioxuslabs/rink) (see [Ink](https://github.com/vadimdemedes/ink))

And finally, some bigger, forward-thinking projects that are too big for a one-person team:

- Completely native renderer for the Dioxus Virtual DOM (see [Flutter](https://flutter.dev/))
- Better support for LiveView
- Code-splitting
- 3D renderer (see [react-three-fiber](https://github.com/pmndrs/react-three-fiber))

Stay tuned for our next article, which will go over some of the optimization techniques that went into making Dioxus blazing fast.

## Community

The future is bright for Rust frontends! If you'd like to get involved, we have a [Discord server](https://discord.gg/XgGxMSkvUM), [a subreddit](http://reddit.com/r/dioxus), and [GitHub discussion pages](https://github.com/DioxusLabs/dioxus/discussions). 

Let us know what you build!

Check out the original `/r/rust` thread here.
