//! A simple little clock that updates the time every few milliseconds.
//!

use dioxus::prelude::*;
use tokio::time::sleep;
use web_time::Instant;

const STYLE: &str = asset!("./examples/assets/clock.css");

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut millis = use_signal(|| 0);

    use_future(move || async move {
        // Save our initial timea
        let start = Instant::now();

        loop {
            sleep(std::time::Duration::from_millis(27)).await;

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
        head::Link { rel: "stylesheet", href: STYLE }
        div { id: "app",
            div { id: "title", "Carpe diem 🎉" }
            div { id: "clock-display", "{time}" }
        }
    }
}
