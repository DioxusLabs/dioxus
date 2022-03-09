# Introducing Dioxus 0.1

After months of work, we're very excited to release the first version of Dioxus!

Dioxus is a new library for building interactive user interfaces with Rust. It is built around a VirtualDOM, making it portable for the web, desktop, server, mobile, and more.

Dioxus has the following design goals:

- **Familiar**: offer a React-like mental model and API surface
- **Correct**: Avoid runtime bugs by moving rules and error handling into the type system
- **Performant**: Scale to the largest of apps for the largest teams
- **Productive:** Comprehensive inline documentation, fast recompiles, and deeply integrated tooling
- **Extensible:** Reusable hooks and components that work on every platform

Dioxus is designed to be familiar for developers comfortable with React paradigms. Our goal is to ensure a smooth transition from TypeScript to Rust without having to learn any major new concepts.  In practice, Rust-Dioxus code looks and feels very similar to TypeScript-React code.

To give you a taste of what Dioxus is all about, here's a simple counter app:

```rust
use dioxus::prelude::*;

fn main() {
	dioxus::desktop::launch(App, |cfg| cfg) 
}

const App: Component<()> = |cx| {
    let mut count = use_state(&cx, || 0);
    cx.render(rsx! {
        h1 { "Count: {count}" }
        button { onclick: move |_| count += 1, "+" }
        button { onclick: move |_| count -= 1, "-" }
    })
};
```

This simple counter is a fully-fledged desktop app, running at native speeds on a native thread. In this particular configuration, Dioxus is using the system's built-in WebView renderer as a "LiveView target." Dioxus automatically shuttles all events from the WebView runtime into the application code. In our app, we can interact natively with system APIs, run multi-threaded code, and do anything a regular native Rust application might do. To publish our app, we simply need to run `cargo build` to compile a portable binary that looks and feels the same on Windows, Mac, and Linux. In fact, our `App` function works exactly the same on desktop, mobile, and the web too.

Dioxus supports everything React does, and more, including

- Server-side-rendering, pre-rendering, and hydration
- Mobile, desktop, and web support
- Suspense, fibers, coroutines, and customizable error handling
- Hooks, first-class state management, components
- Fragments, conditional rendering, and custom elements
- and more!

As a demo, here's a Dioxus app running on all our current supported platforms:

![Untitled](static/Untitled.png)

This very site is built with dioxus, and the source code is available here.

To get started with Dioxus, check out any of the "Getting Started" guides for your platform of choice, or check out the GitHub Repository for more details.

- Getting Started with Dioxus
- Getting Started with Web
- Getting Started with Desktop
- Getting Started with Mobile
- Getting Started with SSR


## Show me some examples of what can be built!


- File explorer desktop app
- Bluetooth scanner desktop app
- IoT management web app
- Chat mobile app
- Hackernews LiveView app


## Why should I use Rust and Dioxus for frontend?


Modern applications are scaling way beyond what our tools originally intended. Unfortunately, these tools make it too easy to fall into a "pit of despair" of buggy, unmaintainable, and fragile code.  Frontend teams are constantly battling technical debt to maintain their velocity; and, while our web tools make it easy and fast to write code, they don't push us to write *better* code. 

We believe that Rust's ability to write high-level, statically typed, and efficient code should make it easier for frontend teams to take on even the most ambitious of projects. Rust projects can be refactored fearlessly: the powerful type system prevents an entire class of bugs at compile time. No more `cannot read property of undefined` ever again! With Rust, all errors must be accounted for at compile time. You cannot ship an app that does not - in some way - handle its errors. 

And while TypeScript is a great addition to JavaScript, it comes with a lot of tweaking flags, a slight performance hit, and an uneven ecosystem where some of the most important packages are not properly typed. TypeScript provides tremendous benefit to JS projects, but comes with its own "TypeScript Tax" that can impede development. Using Rust can be seen as a step up from TypeScript, supporting:

- static types for *all* libraries
- advanced pattern matching
- true immutability by default
- clean, composable iterators
- a consistent module system
- integrated documentation
- inline built-in unit/integration testing
- best-in-class error handling
- simple and fast build system (compared to webpack!)
- powerful standard library and extensive library ecosystem
- various macros (`html!`, `rsx!`) for fast template iteration
- Excellent IDE support for documentation and jump-to-source support

Our goal with Dioxus is to give frontend teams greater confidence in their work. We believe that Dioxus apps are just as ergonomic to write as their React counterparts and may be fearlessly iterated in less time.

However, we do recognize that these benefits are not for everyone, nor do they completely fix "everything that is wrong with frontend development." There are always going to be new patterns, frameworks, and languages that solve these problems better than Rust and Dioxus. We hope that Dioxus serves as a competent companion for developers looking to build reliable and efficient software that extends into the world of user interfaces.

## Show me more:


Here, we'll dive into some features of Dioxus and why it's so fun to use. The API reference serves as a deeper and more comprehensive look at what Dioxus can do.


### **Building a new Project is simple!**


To start a new project, all you need is Cargo (comes with Rust). For a simple desktop app, all we'll need is the `dioxus` create with the appropriate `desktop` feature. In a new crate, we'll just add this to our `Cargo.toml`.

```rust
[dependencies]
dioxus = { version = "*", features = ["desktop"] }
```

Because it's so simple to get started, you probably won't need to reach for a prebuilt template, though we have pre-configured a few templates with suggested project layout.

For web development, you'll want to install the Dioxus CLI to run a local development server and run some basic WASM optimization tools. This can be done with a simple `cargo install dioxus-cli`. The `dioxus-cli` tool will handle building, bundling, development, and optimization for the web and mobile.


### **Multiple flavors of templating: `rsx!`, `html!`, and `factory`, oh my!**


You can use three flavors of templating to declare your UI structure. With the `html!` macro, you get JSX-like functionality. You can copy-paste *most* HTML snippets and expect them to work without modification.

```rust
html! {
	<div> "Hello, world!" </div>
	<button onclick={|_| log::info!("button pressed")}> 
		"Press me"
	</button>
}
```

We also have our own flavor of templating called RSX (a spin on JSX). RSX is very similar to regular struct syntax for Rust so it integrates well with your IDE. RSX supports code-folding, block selection, bracket pair colorizing, autocompletion, symbol renaming - pretty much anything you would expect from writing just regular struct-style code.

```rust
rsx! {
	div { "Hello world" }
	button { onclick: |_| log::info!("button pressed"),
		"Press me"
	}
}
```

If macros aren't your style, then you can always just use the factory API directly:

```rust
LazyNodes(|f| {
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

The `rsx!` and `html!` macros generate idiomatic Rust code that uses the factory API - no different than what you'd write by hand, yourself. Feel free to try it out with `cargo expand` .

To make it easier to work with RSX, we've built a small VSCode extension with useful utilities. This extension provides a command that converts a selected block of HTML into RSX so you can use any web template instantly.


### **Dioxus is perfected for the IDE.**


All Dioxus code operates pleasantly with your IDE. If you really need to write HTML templates, you can use the `html!` macro, but if you're willing to depart from traditional syntax, the `rsx!` macro provides everything you need, and more. 

For starters, all elements are documented through the Rustdoc system - a quick summary of the MDN docs is always under your finger tips:

![static/Screen_Shot_2021-07-06_at_9.42.08_PM.png](static/Screen_Shot_2021-07-06_at_9.42.08_PM.png)

Dioxus also wraps platform-specific events with a custom synthetic event system. This means events enjoy proper autocomplete and documentation, unlike Yew which currently relies WebSys (which is not IDE supported):

![static/Screen_Shot_2021-07-06_at_10.24.03_PM.png](static/Screen_Shot_2021-07-06_at_10.24.03_PM.png)

Even element attributes and event handlers have top-notch documentation!

![static/Screen_Shot_2021-07-07_at_1.21.31_AM.png](static/Screen_Shot_2021-07-07_at_1.21.31_AM.png)

The `rsx!` macro also enjoys code folding, batch renaming, block selection, making most basic code navigation and completion tasks a breeze.

![static/Screen_Shot_2021-07-06_at_10.16.46_PM.png](static/Screen_Shot_2021-07-06_at_10.16.46_PM.png)

Plus, the `rsx!` macro itself is documented, so if you ever forget how to use a certain feature, the documentation is literally right under your cursor:

![static/Screen_Shot_2021-07-07_at_1.28.24_AM.png](static/Screen_Shot_2021-07-07_at_1.28.24_AM.png)

We spent a ton of time on this - we hope you enjoy it!


## **Dioxus is hyperoptimized 🚀🚀🚀**


We take the performance of Dioxus seriously. Instead of resolving to "good enough," Dioxus is designed to push the limits of what a declarative React-like framework can achieve. Dioxus is designed with multi-tenancy in mind: a single machine should be able to run thousands of simultaneous low-latency LiveView apps without skipping a beat. To accomplish this goal we've implemented a large number of optimizations:

- Specialized memory allocators
- Compile-time hashing and diffing hints
- Automatic component memoization
- Cooperative fiber-like scheduling
- DOM Patch Batching


### Bump allocator


Dioxus is incredibly optimized, both for performance and memory efficiency. Due to Rust's type system and low-level abilities, we can precisely control the performance in ways that JavaScript simply cannot. In some aspects, using Rust with Dioxus is faster than plain JavaScript. With direct access over memory allocation and efficient reuse of strings, we can bypass dynamic memory allocation entirely for components after their initial render.

All `Factory` calls use a bump memory allocator to allocate new objects like Elements, Listeners, Attributes, and even strings. Bump memory allocators are the fastest possible memory allocators - significantly faster than the default allocator used in JavaScript runtimes. The original research in bump allocators for Rust-based UI comes from Dodrio (@fitzgen), a now-archived project that demonstrated the insane speeds of low-level memory control.

![static/Screen_Shot_2021-08-17_at_2.24.39_AM.png](static/Screen_Shot_2021-08-17_at_2.24.39_AM.png)


### Static subtree optimization


Because the rsx! macro is deeply integrated with Dioxus, we can apply extremely aggressive optimizations, pushing performance to the absolute maximum. Every rsx! call will generate a static hash, informing the diffing algorithm if the element needs to be checked after it's been mounted. This means that static substructures in complex components will never need to be diffed, saving many precious clock cycles at runtime. For instance, the "div" element in the below example will never be checked once the component has rendered for the first time. Dioxus will only bother computing the differences of attributes and children that it knows *can* change, like the text contents of the paragraph.

```rust
let val = 10;
rsx!{
	div { 
		style: { background_color: "red" }
		"This is a static subtree"
		h1 {"These elements will not be diffed"}
	}
  p { "This content _can_ change: {val}" }
}
```

### Automatic Memoization and Managed Lifetimes


Dioxus provides a very simple form of memoization for components: if a component's props borrow directly from its parent, it is not memoized. This means that Dioxus is the only UI framework in Rust that lets components borrow data from their parents. 

A memoized component is a component that does not borrow any data from its parent. Either it "copies" or "clones" data from the parent into its own props, or generates new data on the fly. These Props must implement `PartialEq` - where you can safely implement your own memoization strategy.

```rust
static App: FC<()> = |cx| rsx!(in cx, Child { name: format!("world") });

#[derive(Props, PartialEq)]
struct ChildProps {
    name: String,
}
static Child: FC<MyProps> = |cx| rsx!(in cx, div {"Hello, {cx.name}"});
```

For components that are not valid for the `'static` lifetime, they do not need to implement `PartialEq` . This lets you choose between dynamic memory allocation or longer diffs. Just like in React, it's better to avoid memory allocation if you know that a child will always change when its parent changes. 

To make components with props that borrow data, we need to use the regular function syntax, and specify the appropriate lifetime:

```rust
struct ChildProps<'a> {
		name: &'a str
}
fn Child<'a>(cx: Scope<'a, ChildProps>) -> Element<'a> {
		rsx!(cx, div {"Hello, {cx.name}"})
}
```

### Cooperative Fiber-Like Scheduling


Rendering with Dioxus is entirely asynchronous and supports the same pause/resume functionality that the new React Fiber rewrite introduced. Internally, Dioxus uses a priority-based scheduler to pause long-running diffs to handle higher-priority work if new events are ready. With the priority scheduler, Dioxus will never block the main thread long enough to cause dropped frames or "jank": event handling is scheduled during idle times and DOM modification is scheduled during animation periods. 

A cool note: Pause/resume uses Rust's efficient Future machinery under the hood, accomplishing the exact same functionality as React fiber with no additional code overhead. 

On top of Pause/Resume, asynchronous rendering enables Suspense, fetch-as-you-render, and even Signals/Streams support.


### Listener Multiplexing / Event delegation


On the web, event delegation is a technique that makes highly-interactive web pages more performant by batching together listeners of the same type. For instance, a single app with more than 100 "onclick" listeners will only have a single listener mounted to the root node. Whenever the listener is triggered, the listener multiplexer will analyze the event and pass it off to the correct listener inside the VirtualDOM. This leads to more performant apps. Dioxus uses an event delegation system, but does not currently have support for proper bubbling or the ability to capture events. Dioxus will always bubble your events (manually) but does not (yet) provide any mechanism to prevent bubbling.


### Patch Batching


When updating the DOM to match your Dioxus component declaration, Dioxus separates its work into two separate phases: diff and commit. During the diff phase, Dioxus will compare an old version of your UI against a new version and figure out exactly which changes need to be made. Unlike other frameworks, Dioxus will not actually modify the DOM during this phase - modifying the DOM is an expensive operation that causes considerable context switching out of "hot codepaths." 

Instead, Dioxus returns a "Mutations" object to the renderer to handle:

```rust
#[derive(Debug)]
pub struct Mutations<'a> {
    pub edits: Vec<DomEdit<'a>>,
    pub noderefs: Vec<NodeRefMutation<'a>>,
}
```

These modifications give the renderer a list of changes that need to be made to modify the real DOM to match the Virtual DOM. Our "DomEdit" type is just a simple enum that can be serialized and sent across the network - making it possible for us to support Liveview and remote clients.

```rust
pub enum DomEdit<'bump> {
    PushRoot { id: u64 },
    PopRoot,
    AppendChildren { many: u32 },
    ReplaceWith { root: u64, m: u32 },
    InsertAfter { root: u64, n: u32 },
		// ....more variants
}
```


### Support for pre-rendering and hydration


As part of the mutation paradigm, Dioxus also supports pre-rendering and hydration of pages  - so you can easily generate your site statically and hydrate it with interactivity once it gets to the client. Rust already runs smoothly on the server, so you can easily build isomorphic apps that achieve amazing Lighthouse scores. Enabling hydration is as simple as:

```rust
// On the server
let pre_rendered = dioxus::ssr::render(App, |cfg| cfg.pre_render(true));

// On the client bundle
dioxus::web::launch(App, |cfg| cfg.hydrate(true));
```


### Support for global shared state


With Dioxus, shared state (React.Context) is simple. We don't need providers, consumers, wrapping components, or anything special. Creating a shared state is as simple as:

```rust
struct SharedState(String);
static App: FC<()> = |cx| {
	// Define it with the dedicated API on Context
	cx.use_create_shared_state(|| SharedState("Hello".to_string()));

	// We can even immediately consume it in the same component
	let my_state = cx.use_shared_state::<SharedState>();
}
```

With Rust's memory safety guarantees and smart pointers, we can share state across the component tree without worrying about accidental mutations or usage errors. 

In fact, Dioxus is shipping with a 1st-class state management solution modeled after Recoil.JS. The name is still in flux, but we're going with `Recoil` for now until we find something better.

In Recoil, shared state is declared with an "Atom" or "AtomFamily". From there, "Selectors" and "SelectorFamilies" make it possible to select, combine, and memoize computations across your app. The two fundamental hooks `use_read` and `use_write` provide an API that matches `use_state` and work across your whole app. The documentation on Recoil is currently very thin, but the API is simple to grok.

```rust
static COUNT: Atom<u32> = |_| 0;

static Incr: FC<()> = |cx| {
    let mut count = use_write(cx, COUNT);
    rsx!(in cx, button { onclick: move |_| count += 1, "increment" })
};

static Decr: FC<()> = |cx| {
    let mut count = use_write(cx, COUNT);
    rsx!(in cx, button { onclick: move |_| count -= 1, "decrement" })
};

static App: FC<()> = |cx| {
    let count = use_read(cx, COUNT);
    rsx!(in cx, "Count is {count}", Incr {}, Decr {})
};
```


### Support for Suspense


Dioxus makes it dead-easy to work with asynchronous values. Simply provide a future to `cx.suspend`, and Dioxus will schedule that future as a task which occurs outside of the regular diffing process. Once the future is complete, that subtree will be rendered. This is a different, more "Rusty" approach to React's "Suspense" mechanism. Because Dioxus uses an asynchronous diffing algorithm, you can easily "fetch as you render." Right now, suspense is very low-level and there aren't a ton of ergonomic hooks built around it. Feel free to make your own!

```rust
#[derive(serde::Deserialize)]
struct DogApi {
    message: String,
}
const ENDPOINT: &str = "https://dog.ceo/api/breeds/image/random";

const App: FC<()> = |cx| {
		let req = use_ref(cx, surf::get(ENDPOINT).recv_json::<DogApi>());

		let doggo = cx.suspend(req, |cx, res| match res {
	      Ok(res) => rsx!(in cx, img { src: "{res.message}" }),
		    Err(_) => rsx!(in cx, div { "No doggos for you :(" }),
		});

    cx.render(rsx!(
        h1 {"Waiting for a doggo..."}
        {doggo}
    ))
};
```


### Built-in coroutines make apps easier to scale


Rust’s async story, while young, does have several highlights. Every future in Rust essentially has a built-in zero-cost AbortHandler - allowing us to pause, start, and restart asynchronous tasks without any additional logic. Dioxus gives full control over these tasks with the use_task hook, making it easy to spawn long-running event loops - essentially coroutines. Coroutines are very popular in game development for their ease of development and high performance even in the largest of apps. In Dioxus, a coroutine would look something like:

```rust
static App: FC<()> = |cx| {
	let websocket_coroutine = use_task(cx, async move {
			let mut socket = connect_to_websocket().await.unwrap();
			while let Some(msg) = socket.recv().await {
					// update our global state
			}
	});
	// render code
};
```

In fact, coroutines allow you to implement true multithreading from within your UI. It's possible to spin up a Tokio/Async_std task or interact with a threadpool from a `use_task` handler.


### Custom hooks


Just like React, Dioxus supports custom hooks. With the `use_hook` method on `Context` , it's easy to write any new hook. However, unlike React, Dioxus manages the mutability of hook data for you, automatically. Calling `use_hook` is basically just adding a field to a managed "bag of state." This lets you obtain an `&mut T` to any data in use-hook - just like if you were writing a regular struct-based component in other frameworks:

```rust
let counter: &mut u32 = cx.use_hook(|_| 0, |val| val, |_| {});
```


### Inline styles


A small feature - but welcome one: Dioxus supports inline styles! This gives you 3 ways of styling components: dedicated CSS files, through `style` tags, and inline styles. Here:

```rust
// dedicated file
link { rel: "stylesheet", href: "style.css" }

// style tags
let style = include_str!("style.css");
rsx!(style { "{style}" });

// inline styles
rsx!(div { background_color: "red" });
```

Right now, a dedicated "Style" object is not supported for style merging, but we plan to add it in the future.


### Works on mobile and desktop


We’ve mentioned before that Dioxus works practically anywhere that Rust works. When running natively as a desktop or mobile app, your Dioxus code will run on its own thread: not inside of a web runtime. This means you can access hardware, file system, and platform APIs directly without needing to go through a shim layer. In our examples, we feature a file explorer app and Bluetooth scanner app where platform access occurs inside an asynchronous multithreaded coroutine. This solves the problem faced by React Native and other cross-platform toolkits where JavaScript apps occur a massive performance penalty with substantial maintenance overhead associated with platform API shims.

Using Dioxus on mobile is easy:

```rust
dioxus::mobile::launch(App, |cfg| cfg);
```

However, be warned that mobile is considered very experimental and there will likely be quirks. Dioxus is leveraging work done by the Tauri team to enable mobile support, and mobile support isn't technically complete in Tauri - yet. iOS should be supported out of the box, but Android support will take custom some boilerplate that hasn't been figured out completey.


## FAQ:


*"I thought the overhead of Rust to JS makes Rust apps slow?"*

Wasm-bindgen is *just* as fast Vanilla JS, beating out nearly every JS framework in the [framework benchmark](https://krausest.github.io/js-framework-benchmark/2021/table_chrome_91.0.4472.77.html). The biggest bottleneck of Rust interacting with JS is the overhead of translating Rust's UTF-8 strings into JS's UTF-16 strings. Dioxus uses string-interning (caching) and Bloomfilters to effectively circumvent this overhead, meaning even this bottleneck is basically invisible. In fact, Dioxus actually beats the wasm-bindgen benchmark, approaching near vanilla-JS speeds on the JS framework benchmark.

*"Isn't it much more difficult to write Rust than JS?"*

Frankly, the type of code used to write UI is not that complex. When dealing with complex problems, Rust code may end up more complex. React, Redux, and Immer all struggle with mutability issues that Rust naturally prevents. With Rust, it's impossible to accidentally mutate a field, saving developers not only from memory safety bugs, but logic bugs. Plus, we truly believe that Dioxus code will be even easier to write and maintain than its JS counterpart:

```rust
// The Rust-Dioxus version
const App: FC<()> = |cx| {
    let mut count = use_state(&cx, || 0);
    cx.render(rsx!{
				h1 { "Count: {count}" }
				button { onclick: move |_| count += 1, "+" }
				button { onclick: move |_| count -= 1, "-" }
		})
}

// The TypeScript-React version:
const App: FC<()> = (props) => {
	let [count, set_count] = use_state(0);
	return (
		<>
				<h1> Count: {count} </h1>	
        <button onclick={() => set_count(count + 1)}> "+" </button>
        <button onclick={() => set_count(count - 1)}> "-" </button>			
		</>
	);
};
```

"Doesn't Rust take forever to compile?"

Have you ever used Webpack? 🙂 It's not uncommon for a large Webpack builds to push 3-5 minutes with hot iterations in 20-30 seconds. We've found that Rust's compiler has gotten much faster than it once was, WASM projects have fewer big dependencies, and the new Cranelift backend generates WASM code at blindingly-fast speeds. Smaller WASM projects will compile nearly instantly and the bigger projects might take 5-10 seconds for a hot-reload.

"Aren't Rust binaries too big for the web?"

Dioxus' gzipped "Hello world" clocks in at around 50 kB - more than Preact/Inferno but on par with React. However, WASM code is compiled as it is downloaded (in parallel) - and it compiles incredibly quickly!  By the time the 50 kB is downloaded, the app is already running at full speed. In this way, it's hard to compare WASM and JS; JS needs time to be JIT-ed and cannot be JIT-ed until after it's downloaded completely. Typically, the JIT process takes much longer than the WASM compile process, so it's hard compare kilobytes to kilobytes. 

In short - WASM apps launch just as fast, if not faster, than JS apps - which is typically the main concern around code size.


## What's on the Roadmap?


The world of Rust on the frontend is barely explored. Given the performance, ergonomics, and portability of Dioxus, we expect there to be a ton of different applications where having a React-like toolkit running natively can enable things previously impossible. 

In the coming weeks, the plan is to finish the final outstanding features where Dioxus is lacking in comparison to React:

- Synchronous and asynchronous layout-related effects like `useEffect`
- Support for event "capturing" and real event bubbling
- Transition Effects for suspense
- Micro-optimizations and better cross-platform/browser bug mitigations
- Hooks to guide the diffing algorithm
- Better support for subtree memoization
- More thorough documentation, fleshing out sore spots

We also need some help in important crates currently missing:

- 1st class cross-platform router
- An extension to DioxusStudio that enables lazy bundling of static assets
- Animation library (like React Spring)
- Better support for Dioxus as a TUI framework

And finally, some bigger, forward-thinking projects that are too big for one person:

- Completely native renderer for the Dioxus VirtualDOM (like Flutter)
- Better support for LiveView
- Code-splitting
- 3D renderer like React-three-fiber


## Community


The future is bright for Rust frontends! If you'd like to get involved, we have a 

- Discord
- Subreddit
- Github Discussions page

Check out the original `r/rust` thread here.

Let us know what you build!
