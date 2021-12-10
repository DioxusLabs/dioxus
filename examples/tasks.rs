//! Example: README.md showcase
//!
//! The example from the README.md.

use std::time::Duration;

use dioxus::prelude::*;
fn main() {
    dioxus::desktop::launch(App, |c| c);
}

static App: Component<()> = |cx, props| {
    let mut count = use_state(cx, || 0);

    cx.push_task(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        println!("setting count");
        count += 1;
        // count.set(10);
        // *count += 1;
        // let c = count.get() + 1;
        // count.set(c);
    });

    cx.render(rsx! {
        div {
            h1 { "High-Five counter: {count}" }
            // button {
            //     onclick: move |_| count +=1 ,
            //     "Click me!"
            // }
        }
    })
};
