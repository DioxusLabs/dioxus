//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;
fn main() {
    dioxus::desktop::launch(App, |c| c);
}

static App: FC<()> = |cx, props| {
    let mut count = use_state(cx, || 0);

    cx.push_task(async {
        panic!("polled future");
        //
    });

    cx.render(rsx! {
        div {
            h1 { "High-Five counter: {count}" }
        }
    })
};
