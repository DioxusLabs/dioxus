//! A simple little clock that updates the time every few milliseconds.
//!
//! Neither Rust nor Tokio have an interval function, so we just sleep until the next update.
//! Tokio timer's don't work on WASM though, so you'll need to use a slightly different approach if you're targeting the web.

use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut millis = use_signal(|| 0);

    use_future(move || async move {
        // Save our initial timea
        let start = std::time::Instant::now();

        loop {
            // In lieu of an interval, we just sleep until the next update
            let now = tokio::time::Instant::now();
            tokio::time::sleep_until(now + std::time::Duration::from_millis(27)).await;

            // Update the time, using a more precise approach of getting the duration since we started the timer
            millis.set(start.elapsed().as_millis() as i64);
        }
    });

    // Format the time as a string
    // This is rather cheap so it's fine to leave it in the render function
    let time = format!(
        "{:02}:{:02}:{:03}",
        millis() / 1000 / 60 % 60,
        millis() / 1000 % 60,
        millis() % 1000
    );

    rsx! {
        style { {include_str!("./assets/clock.css")} }
        div { id: "app",
            div { id: "title", "Carpe diem 🎉" }
            div { id: "clock-display", "{time}" }
        }
    }
}
