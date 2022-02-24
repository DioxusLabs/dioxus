use bevy::prelude::*;
use dioxus::desktop::{DioxusDesktopContext, DioxusDesktopPlugin};
use dioxus::prelude::*;

fn main() {
    App::new()
        .insert_resource(DioxusDesktopContext {
            root: app,
            props: (),
        })
        .add_plugin(DioxusDesktopPlugin)
        .run();
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            h1 { "Bevy Plugin Example" }
        }
    })
}
