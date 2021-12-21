use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;

use dioxus_html as dioxus_elements;

fn main() {
    dioxus_desktop::launch(App);
}

static App: Component<()> = |cx| {
    cx.render(rsx!(
        div {
            "hello world!"
        }
        {(0..10).map(|f| rsx!( div {"abc {f}"}))}
    ))
};
