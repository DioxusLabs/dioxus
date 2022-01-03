# TUI

TUI support is currently quite experimental. Even the project name will change. But, if you're willing to venture into the realm of the unknown, this guide will get you started.


[TUI Support](https://github.com/DioxusLabs/rink/raw/master/examples/example.png)


## Getting Set up


To tinker with TUI support, start by making a new package and adding our TUI package from git.

```shell
$ cargo new --bin demo
$ cd demo
$ cargo add dioxus
$ cargo add rink --git https://github.com/DioxusLabs/rink.git
```



Then, edit your `main.rs` with the basic template. 

```rust
//  main
use dioxus::prelude::*;

fn main() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild();

    rink::render_vdom(&mut dom).unwrap();
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

Press "q" to close the app (yes, this is hardcoded, we are working on handlers to expose this in your code.)

## Notes

- Our TUI package uses flexbox for layout
- 1px is one character lineheight. Your regular CSS px does not translate.
- If your app panics, your terminal is wrecked. This will be fixed eventually.

## Future Steps

Make sure to read the [Dioxus Guide](https://dioxuslabs.com/guide) if you already haven't!
