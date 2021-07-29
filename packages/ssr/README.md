# Dioxus SSR

Render a Dioxus VirtualDOM to a string.

```rust
// Our app:
const App: FC<()> = |cx| cx.render(rsx!(div {"hello world!"}));

// Build the virtualdom from our app
let mut vdom = VirtualDOM::new(App);

// This runs components, lifecycles, etc. without needing a physical dom. Some features (like noderef) won't work.
vdom.rebuild_in_place();

// Render the entire virtualdom from the root
let text = dioxus_ssr::render_root(&vdom);
assert_eq!(text, "<div>hello world!</div>")
```



## Pre-rendering


