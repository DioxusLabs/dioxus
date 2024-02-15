//! Managing focus
//!
//! This example shows how to manage focus in a Dioxus application. We implement a "roulette" that focuses on each input
//! in the grid every few milliseconds until the user interacts with the inputs.

use std::rc::Rc;

use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    // Element data is stored as Rc<MountedData> so we can clone it and pass it around
    let mut elements = use_signal(Vec::<Rc<MountedData>>::new);
    let mut running = use_signal(|| true);

    use_future(move || async move {
        let mut focused = 0;

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            if !running() {
                continue;
            }

            if let Some(element) = elements.with(|f| f.get(focused).cloned()) {
                _ = element.set_focus(true).await;
            } else {
                focused = 0;
            }

            focused += 1;
        }
    });

    rsx! {
        style { {include_str!("./assets/roulette.css")} }
        h1 { "Input Roulette" }
        button { onclick: move |_| running.toggle(), "Toggle roulette" }
        div { id: "roulette-grid",
            // Restart the roulette if the user presses escape
            onkeydown: move |event| {
                if event.code().to_string() == "Escape" {
                    running.set(true);
                }
            },

            // Draw the grid of inputs
            for i in 0..100 {
                input {
                    r#type: "number",
                    value: "{i}",
                    onmounted: move |cx| elements.write().push(cx.data()),
                    oninput: move |_| running.set(false),
                }
            }
        }
    }
}
