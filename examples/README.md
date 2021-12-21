# Examples

Most of these examples are run through webview so you don't need the Dioxus CLI installed to preview the functionality.

These examples are fully-fledged micro apps. They can be ran with the `cargo run --example XYZ`

| Example                                             | What it does                                | Status |
| --------------------------------------------------- | ------------------------------------------- | ------ |
| [The basics](./basics.rs)                           | A few basic examples to preview Dioxus      | 🛠      |
| [fine grained reactivity](./signals.rs)             | Escape `diffing` by writing values directly | 🛠      |
| [Global State Management](./statemanagement.rs)     | Share state between components              | 🛠      |
| [Virtual Refs]()                                    | Cross-platform imperative elements          | 🛠      |
| [Inline Styles](./inline-styles.rs)                 | Define styles for elements inline           | 🛠      |
| [Conditional Rendering](./conditional-rendering.rs) | Hide/Show elements using conditionals       | ✅      |

These examples are not necessarily meant to be run, but rather serve as a reference for the given functionality.

| Example                                             | What it does                                    | Status |
| --------------------------------------------------- | ----------------------------------------------- | ------ |
| [The basics](./basics.rs)                           | A few basic examples to preview Dioxus          | 🛠      |
| [fine grained reactivity](./signals.rs)             | Escape `diffing` by writing values directly     | 🛠      |
| [Global State Management](./statemanagement.rs)     | Share state between components                  | 🛠      |
| [Virtual Refs]()                                    | Cross-platform imperative elements              | 🛠      |
| [Inline Styles](./inline-styles.rs)                 | Define styles for elements inline               | 🛠      |
| [Conditional Rendering](./conditional-rendering.rs) | Hide/Show elements using conditionals           | ✅      |
| [Maps/Iterators](./iterators.rs)                    | Use iterators in the rsx! macro                 | ✅      |
| [Render To string](./tostring.rs)                   | Render a mounted virtualdom to a string         | 🛠      |
| [Component Children](./children.rs)                 | Pass children into child components             | 🛠      |
| [Function Driven children]()                        | Pass functions to make VNodes                   | 🛠      |
| [Memoization & Borrowed Data](./memo.rs)            | Suppress renders, borrow from parents           | ✅      |
| [Fragments](./fragments.rs)                         | Support root-less element groups                | ✅      |
| [Null/None Components](./empty.rs)                  | Return nothing!                                 | 🛠      |
| [Spread Pattern for props](./spreadpattern.rs)      | Manually specify and override props             | ✅      |
| [Controlled Inputs](./controlled-inputs.rs)         | this does                                       | 🛠      |
| [Custom Elements]()                                 | Define custom elements                          | 🛠      |
| [Web Components]()                                  | Custom elements to interface with WebComponents | 🛠      |
| [Testing And debugging]()                           | this does                                       | 🛠      |
| [Asynchronous Data]()                               | Using suspense to wait for data                 | 🛠      |
| [Fiber/Scheduled Rendering]()                       | this does                                       | 🛠      |
| [CSS Compiled Styles]()                             | this does                                       | 🛠      |
| [Anti-patterns](./antipatterns.rs)                  | A collection of discouraged patterns            | ✅      |
| [Complete rsx reference](./rsx_usage.rs)            | A complete reference for all rsx! usage         | ✅      |
| [Event Listeners](./listener.rs)                    | Attach closures to events on elements           | ✅      |

These web-specific examples must be run with `dioxus-cli` using `dioxus develop --example XYZ`

| Example | What it does |
| ------- | ------------ |
| asd     | this does    |
| asd     | this does    |



## Show me some examples!

In our collection of examples, guides, and tutorials, we have:
- The book (an introductory course)
- The guide (an in-depth analysis of everything in Dioxus)
- The reference (a collection of examples with heavy documentation)
- The general examples
- The platform-specific examples (web, ssr, desktop, mobile, server)
  
Here's what a few common tasks look like in Dioxus:

Nested components with children and internal state:
```rust
fn App(cx: Context, props: &()) -> Element {
  cx.render(rsx!( Toggle { "Toggle me" } ))
}

#[derive(PartialEq, Props)]
struct ToggleProps { children: Element }

fn Toggle(cx: Context, props: &ToggleProps) -> Element {
  let mut toggled = use_state(&cx, || false);
  cx.render(rsx!{
    div {
      {&props.children}
      button { onclick: move |_| toggled.set(true),
        {toggled.and_then(|| "On").or_else(|| "Off")}
      }
    }
  })
}
```

Controlled inputs:
```rust
fn App(cx: Context, props: &()) -> Element {
  let value = use_state(&cx, String::new);
  cx.render(rsx!( 
    input {
      "type": "text",
      value: "{value}",
      oninput: move |evt| value.set(evt.value.clone())
    }
  ))
}
```

Lists and Conditional rendering:
```rust
fn App(cx: Context, props: &()) -> Element {
  let list = (0..10).map(|i| {
    rsx!(li { key: "{i}", "Value: {i}" })
  });
  
  let title = match list.len() {
    0 => rsx!("Not enough"),
    _ => rsx!("Plenty!"),
  };

  if should_show {
    cx.render(rsx!( 
      {title}
      ul { {list} } 
    ))
  } else {
    None
  }
}
```

Tiny components:
```rust
static App: Component<()> = |cx, _| rsx!(cx, div {"hello world!"});
```

Borrowed prop contents:
```rust
fn App(cx: Context, props: &()) -> Element {
  let name = use_state(&cx, || String::from("example"));
  rsx!(cx, Child { title: name.as_str() })
}

#[derive(Props)]
struct ChildProps<'a> { title: &'a str }

fn Child(cx: Context, props: &ChildProps) -> Element {
  rsx!(cx, "Hello {cx.props.title}")
}
```

Global State
```rust
struct GlobalState { name: String }

fn App(cx: Context, props: &()) -> Element {
  use_provide_shared_state(cx, || GlobalState { name: String::from("Toby") })
  rsx!(cx, Leaf {})
}

fn Leaf(cx: Context, props: &()) -> Element {
  let state = use_consume_shared_state::<GlobalState>(cx)?;
  rsx!(cx, "Hello {state.name}")
}
```

Router (inspired by Yew-Router)
```rust
#[derive(PartialEq, Clone,  Hash, Eq, Routable)]
enum Route {
  #[at("/")]
  Home,
  #[at("/post/{id}")]
  Post(id)
}

fn App(cx: Context, props: &()) -> Element {
  let route = use_router(cx, Route::parse);
  cx.render(rsx!(div {
    {match route {
      Route::Home => rsx!( Home {} ),
      Route::Post(id) => rsx!( Post { id: id })
    }}
  }))  
}
```

Suspense 
```rust
fn App(cx: Context, props: &()) -> Element {
  let doggo = use_suspense(cx,
    || async { reqwest::get("https://dog.ceo/api/breeds/image/random").await.unwrap().json::<Response>().await.unwrap() },
    |response| cx.render(rsx!( img { src: "{response.message}" }))
  );
  
  cx.render(rsx!{
    div {
      "One doggo coming right up:"
      {doggo}
    }
  })
}
```
