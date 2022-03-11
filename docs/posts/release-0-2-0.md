# Dioxus v0.2 Release: TUI, Router, Fermi, and Tooling

> March 9, 2022

Thanks to these amazing folks for their financial support on OpenCollective:

- [@t1m0t](https://github.com/t1m0t)
- [@alexkirsz](https://github.com/t1m0t)
- [@freopen](https://github.com/freopen)
- [@DannyMichaels](https://github.com/DannyMichaels)
- [@SweetLittleMUV](https://github.com/Fatcat560)

Thanks to these amazing folks for their code contributions:

- [@mrxiaozhuox](https://github.com/mrxiaozhuox)
- [@autarch](https://github.com/autarch)
- [@FruitieX](https://github.com/FruitieX)
- [@t1m0t](https://github.com/t1m0t)
- [@Demonthos](https://github.com/Demonthos)
- [@oovm](https://github.com/oovm)
- [@6asaaki](https://github.com/6asaaki)


Just over two months in, and we already a ton of awesome changes to Dioxus!

Dioxus is a recently-released library for building interactive user interfaces (GUI) with Rust. It is built around a Virtual DOM, making it portable for the web, desktop, server, mobile, and more. Dioxus looks and feels just like React, so if you know React, then you'll feel right at home.

```rust
fn app(cx: Scope) -> Element {
    let mut count = use_state(&cx, || 0);

    cx.render(rsx! {
        h1 { "Count: {count}" }
        button { onclick: move |_| count += 1, "+" }
        button { onclick: move |_| count -= 1, "-" }
    })
}
```

# What's new?

A *ton* of stuff happened in this release; 550+ commits, 23 contributors, 2 minor releases, and 6 backers on Open Collective.

Some of the major new features include:

- We now can render into the terminal, similar to Ink.JS - a huge thanks to [@Demonthos](https://github.com/Demonthos)
- We have a new router in the spirit of React-Router [@autarch](https://github.com/autarch)
- We now have Fermi for global state management in the spirit of [Recoil.JS](https://recoiljs.org)
- Our desktop platform got major upgrades, getting closer to parity with Electron [@mrxiaozhuox](https://github.com/mrxiaozhuox)
- Our CLI tools now support HTML-to-RSX translation for converting 3rd party HTML into Dioxus [@mrxiaozhuox](https://github.com/mrxiaozhuox)
- Dioxus-Web is sped up by 2.5x with JS-based DOM manipulation (3x faster than React)

We also fixed and improved a bunch of stuff - check out the full list down below.


## A New Renderer: Your terminal!

When Dioxus was initially released, we had very simple support for logging Dioxus elements out as TUI elements. In the past month or so, [@Demonthos](https://github.com/Demonthos) really stepped up and made the new crate a reality.


![Imgur](https://i.imgur.com/GL7uu3r)

The new TUI renderer even supports mouse movements, keyboard input, async tasks, borders, and a ton more.

<blockquote class="imgur-embed-pub" lang="en" data-id="a/49n7Hj2" data-context="false" ><a href="//imgur.com/a/49n7Hj2"></a></blockquote><script async src="//s.imgur.com/min/embed.js" charset="utf-8"></script>



## New Router

We totally revamped the router, switching away from the old yew-router approach to the more familiar [React-Router](http://reactrouter.com). It's less type-safe but provides more flexibility and support for beautiful URLs.

Apps with routers are *really* simple now. It's easy to compose the "Router", a "Route", and "Links" to define how your app is laid out:

```rust
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            onchange: move |_| log::info!("Route changed!"),
            ul {
                Link { to: "/",  li { "Go home!" } }
                Link { to: "users",  li { "List all users" } }
                Link { to: "blog", li { "Blog posts" } }
            }
            Route { to: "/", "Home" }
            Route { to: "/users", "User list" }
            Route { to: "/users/:name", User {} }
            Route { to: "/blog", "Blog list" }
            Route { to: "/blog/:post", BlogPost {} }
            Route { to: "", "Err 404 Route Not Found" }
        }
    })
}
```

We're also using hooks to parse the URL parameters and segments so you can interact with the router from anywhere deeply nested in your app.

```rust
#[derive(Deserialize)]
struct Query { name: String }

fn BlogPost(cx: Scope) -> Element {
    let post = use_route(&cx).segment("post")?;
    let query = use_route(&cx).query::<Query>()?;

    cx.render(rsx!{
        "Viewing post {post}"
        "Name selected: {query}"
    })
}
```

Give a big thanks to [@autarch](https://github.com/autarch) for putting in all the hard work to make this new router a reality.

The Router guide is [available here](https://dioxuslabs.com/nightly/router/) - thanks to [@dogedark](https://github.com/dogedark).

## Fermi for Global State Management

Managing state in your app can be challenging. Building global state management solutions can be even more challenging. For the first big attempt at building a global state management solution for Dioxus, we chose to keep it simple and follow in the footsteps of the [Recoil.JS](http://recoiljs.org) project.

Fermi uses the concept of "Atoms" for global state. These individual values can be get/set from anywhere in your app. Using state with Fermi is basically as simple as `use_state`.

```rust
// Create a single value in an "Atom"
static TITLE: Atom<&str> = |_| "Hello";

// Read the value from anywhere in the app, subscribing to any changes
fn app(cx: Scope) -> Element {
    let title = use_read(&cx, TITLE);
    cx.render(rsx!{
        h1 { "{title}" }
        Child {}
    })
}

// Set the value from anywhere in the app
fn Child(cx: Scope) -> Element {
    let set_title = use_set(&cx, TITLE);
    cx.render(rsx!{
        button {
            onclick: move |_| set_title("goodbye"),
            "Say goodbye"
        }
    })
}
```

## Inline Props Macro

For internal components, explicitly declaring props structs can become tedious. That's why we've built the new `inline_props` macro. This macro lets you inline your props definition right into your component function arguments.

Simply add the `inline_props` macro to your component:
```rust
#[inline_props]
fn Child<'a>(
    cx: Scope,
    name: String,
    age: String,
    onclick: EventHandler<'a, ClickEvent>
) -> Element {
    cx.render(rsx!{
        button {
            "Hello, {name}"
            "You are {age} years old"
            onclick: move |evt| onclick.call(evt)
        }
    })
}
```

You won't be able to document each field or attach attributes so you should refrain from using it in libraries.

## Props optional fields

Sometimes you don't want to specify *every* value in a component's props, since there might a lot. That's why the `Props` macro now supports optional fields. You can use a combination of `default`, `strip_option`, and `optional` to tune the exact behavior of properties fields.

```rust
#[derive(Props, PartialEq)]
struct ChildProps {
    #[props(default = "client")]
    name: String,

    #[props(default)]
    age: Option<u32>,

    #[props(optional)]
    age: Option<u32>,
}

// then to use the accompanying component
rsx!{
    Child {
        name: "asd",
    }
}
```

## Dioxus Web Speed Boost

We've changed how DOM patching works in Dioxus-Web; now, all of the DOM manipulation code is written in TypeScript and shared between our web, desktop, and mobile runtimes.

On an M1-max, the "create-rows" operation used to take 45ms. Now, it takes a mere 17ms - 3x faster than React. We expect an upcoming optimization to bring this number as low as 3ms.

Under the hood, we have a new string interning engine to cache commonly used tags and values on the Rust <-> JS boundary, resulting in significant performance improvements.

Overall, Dioxus apps are even more snappy than before.


Before and after:
![Before and After](https://imgur.com/byTBGlO)


## Dioxus Desktop Window Context

A very welcome change, thanks AGAIN to [@mrxiaozhuox](https://github.com/mrxiaozhuox) is support for imperatively controlling the desktop window from your Dioxus code.

A bunch of new methods were added:
- Minimize and maximize window
- Close window
- Focus window
- Enable devtools on the fly

And more!

In addition, Dioxus Desktop now autoresolves asset locations, so you can easily add local images, JS, CSS, and then bundle it into an .app without hassle.

You can now build entirely borderless desktop apps:

![img](https://i.imgur.com/97zsVS1)

<!-- ## VSCode Extension

To make life easier and improve your development experience, we've launched the first iteration of the official Dioxus VSCode extension. If you're not using VSCode, you can still take advantage of these new features through the CLI tool.

Included in the new extension is:

- Auto-formatting of `rsx!` blocks
- Convert selection of HTML to RSX
- Extract RSX as component

[To install the extension, go here](https://marketplace.visualstudio.com/items?itemName=matklad.rust-analyzer).

The VSCode extension is really easy to contribute to and has tons of potential. This is a great place to start contributing to the Dioxus project *and* improve your development experience. -->

## CLI Tool

Thanks to the amazing work by [@mrxiaozhuox](https://github.com/mrxiaozhuox), our CLI tool is fixed and working better than ever. The Dioxus-CLI sports a new development server, an HTML to RSX translation engine, a `cargo fmt`-style command, a configuration scheme, and much more.

Unlike its counterpart, `Trunk.rs`, the dioxus-cli supports running examples and tests, making it easier to test web-based projects and showcase web-focused libraries.

## Async Improvements

Working with async isn't the easiest part of Rust. To help improve things, we've upgraded async support across the board in Dioxus.


First, we upgraded the `use_future` hook. It now supports dependencies, which let you regenerate a future on the fly as its computed values change. It's never been easier to add datafetching to your Rust Web Apps:

```rust
fn RenderDog(cx: Scope, breed: String) -> Element {
    let dog_request = use_future(&cx, (breed,), |(breed,)| async move {
        reqwest::get(format!("https://dog.ceo/api/breed/{}/images/random", breed))
            .await
            .unwrap()
            .json::<DogApi>()
            .await
    });

    cx.render(match dog_request.value() {
        Some(Ok(url)) => rsx!{ img { url: "{url}" } },
        Some(Err(url)) => rsx!{ span { "Loading dog failed" }  },
        None => rsx!{ "Loading dog..." }
    })
}
```

Additionally, we added better support for coroutines. You can now start, stop, resume, and message with asynchronous tasks. The coroutine is automatically exposed to the rest of your app via the Context API. For the vast majority of apps, Coroutines can satisfy all of your state management needs:

```rust
fn App(cx: Scope) -> Element {
    let sync_task = use_coroutine(&cx, |rx| async move {
        connect_to_server().await;
        let state = MyState::new();

        while let Some(action) = rx.next().await {
            reduce_state_with_action(action).await;
        }
    });

    cx.render(rsx!{
        button {
            onclick: move |_| sync_task.send(SyncAction::Username("Bob")),
            "Click to sync your username to the server"
        }
    })
}
```

## All New Features

We've covered the major headlining features, but there were so many more!

- [x] A new router @autarch
- [x] Fermi for global state management
- [x] Translation of docs and Readme into Chinese @mrxiaozhuox
- [x] 2.5x speedup by using JS-based DOM manipulation (3x faster than React)
- [x] Beautiful documentation overhaul
- [x] InlineProps macro allows definition of props within a component's function arguments
- [x] Improved dev server, hot reloading for desktop and web apps [@mrxiaozhuox](https://github.com/mrxiaozhuox)
- [x] Templates: desktop, web, web/hydration, Axum + SSR, and more [@mrxiaozhuox](https://github.com/mrxiaozhuox)
- [x] Web apps ship with console_error_panic_hook enabled, so you always get tracebacks
- [x] Enhanced Hydration and server-side-rendering (recovery, validation)
- [x] Optional fields for component properties
- [x] Introduction of the `EventHandler` type
- [x] Improved use_state hook to be closer to react
- [x] Improved use_ref hook to be easier to use in async contexts
- [x] New use_coroutine hook for carefully controlling long-running async tasks
- [x] Prevent Default attribute
- [x] Provide Default Context allows injection of global contexts to the top of the app
- [x] push_future now has a spawn counterpart to be more consistent with rust
- [x] Add gap and gap_row attributes [@FruitieX](https://github.com/FruitieX)
- [x] File Drag n Drop support for Desktop
- [x] Custom handler support for desktop
- [x] Forms now collect all their values in oninput/onsubmit
- [x] Async tasks now are dropped when components unmount
- [x] Right-click menus are now disabled by default

## Fixes
- [x] Windows support improved across the board
- [x] Linux support improved across the board
- [x] Bug in Calculator example
- [x] Improved example running support

A ton more! Dioxus is now much more stable than it was at release!

## Community Additions
- [Styled Components macro](https://github.com/Zomatree/Revolt-Client/blob/master/src/utils.rs#14-27) [@Zomatree](https://github.com/Zomatree)
- [Dioxus-Websocket hook](https://github.com/FruitieX/dioxus-websocket-hooks) [@FruitieX](https://github.com/FruitieX)
- [Home automation server app](https://github.com/FruitieX/homectl) [@FruitieX](https://github.com/FruitieX)
- [Video Recording app](https://github.com/rustkid/recorder)
- [Music streaming app](https://github.com/autarch/Crumb/tree/master/web-frontend) [@autarch](https://github.com/autarch)
- [NixOS dependency installation](https://gist.github.com/FruitieX/73afe3eb15da45e0e05d5c9cf5d318fc) [@FruitieX](https://github.com/FruitieX)
- [Vercel Deploy Template](https://github.com/lucifer1004/dioxus-vercel-demo) [@lucifer1004](https://github.com/lucifer1004)
- [Render Katex in Dioxus](https://github.com/oovm/katex-wasm)
- [Render PrismJS in Dioxus](https://github.com/oovm/prism-wasm)
- [Compile-time correct TailwindCSS](https://github.com/houseabsolute/tailwindcss-to-rust)
- [Autogenerate tailwind CSS](https://github.com/oovm/tailwind-rs)
- [Heroicons library](https://github.com/houseabsolute/dioxus-heroicons)
- [RSX -> HTML translator app](https://dioxus-convert.netlify.app)
- [Toast Support](https://github.com/mrxiaozhuox/dioxus-toast)
- New Examples: forms, routers, linking, tui, and more!

## Looking Forward

Dioxus is still under rapid, active development. We'd love for you to get involved! For the next release, we're looking to add:

- A query library like react-query
- Native WGPU renderer support
- Multiwindow desktop app support
- Full LiveView integrations for Axum, Warp, and Actix
- A builder pattern for elements (no need for rsx!)
- Autoformatting of rsx! code (like cargo fmt)
- Beef up and publish the VSCode Extension
