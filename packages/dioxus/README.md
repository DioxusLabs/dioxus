# Dioxus

This crate provides all the batteries required to build Dioxus apps.

Included in this crate is:
- Dioxus core
- Essential hooks (use_state, use_ref, use_reducer, etc)
- rsx! and html! macros

You'll still need to pull in a renderer to render the Dioxus VDOM. Any one of:
- dioxus-web (to run in WASM)
- dioxus-ssr (to run on the server or for static sites)
- dioxus-webview (to run on the desktop)
- dioxus-mobile (to run on iOS/Android)


Make sure dioxus and its renderer share the same major version; the renderers themselves rely on dioxus.
```toml
[dependencies]
dioxus = "0.2" 
dioxus-web = "0.2" 
```

```rust
use dioxus::*;

fn main() {
    dioxus_web::start(|ctx, _| {
        rsx!{in ctx, div { "Hello world" }}
    })
}
```



Additionally, you'll want to look at other projects for more batteries
- essential-hooks (use_router, use_storage, use_cache, use_channel)
- Recoil.rs or Reducer.rs for state management
- 3D renderer (ThreeD), charts (Sciviz), game engine (Bevy)


Extra resources:
- The guide is available at:
- The crate docs are at:
- Video tutorials are at:
- Examples are at:
- Full clones are at:
- Community at: [www.reddit.com/r/dioxus]()

Happy building!
