# Getting Started: TUI

TUI support is currently quite experimental. Even the project name will change. But, if you're willing to venture into the realm of the unknown, this guide will get you started.


[TUI Support](https://github.com/DioxusLabs/rink/raw/master/examples/example.png)


## Getting Set up


To tinker with TUI support, start by making a new package and adding our TUI feature.

```shell
$ cargo new --bin demo
$ cd demo
$ cargo add dioxus --features tui
```



Then, edit your `main.rs` with the basic template. 

```rust
//  main
use dioxus::prelude::*;

fn main() {
    dioxus::tui::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",

            "Hello world!"
        }
    })
}
```

To run our app:

```shell
$ cargo run
```

Press "ctrl-c" to close the app. To switch from "ctrl-c" to  just "q" to quit you can launch the app with a Configeration to disable the default quit and use the root TuiContext to quit on your own.

```rust
//  main
use dioxus::events::{KeyCode, KeyboardEvent};
use dioxus::prelude::*;
use dioxus::tui::TuiContext;

fn main() {
    dioxus::tui::launch_cfg(
        app,
        dioxus::tui::Config {
            ctrl_c_quit: false,
            // Some older terminals only support 16 colors or ANSI colors if your terminal is one of these change this to BaseColors or ANSI
            rendering_mode: dioxus::tui::RenderingMode::Rgb,
        },
    );
}

fn app(cx: Scope) -> Element {
    let tui_ctx: TuiContext = cx.consume_context().unwrap();

    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",
            onkeydown: move |k: KeyboardEvent| if let KeyCode::Q = k.data.key_code {
                tui_ctx.quit();
            },

            "Hello world!"
        }
    })
}
```

## Notes

- Our TUI package uses flexbox for layout
- 1px is one character lineheight. Your regular CSS px does not translate.
- If your app panics, your terminal is wrecked. This will be fixed eventually.

## Future Steps

Make sure to read the [Dioxus Guide](https://dioxuslabs.com/guide) if you already haven't!
