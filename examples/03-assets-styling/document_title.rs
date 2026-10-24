//! Setting the page title.
//!
//! The `Title` component renders a `<title>` tag into the document head. On the web it
//! sets the browser tab title; on desktop it becomes the window title. Because it's a
//! regular component, its contents can be driven by signals and update live.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut unread = use_signal(|| 0);

    rsx! {
        // The title updates every time the signal changes
        Title {
            if unread() > 0 { "({unread}) Inbox" } else { "Inbox" }
        }

        h1 { "Inbox" }
        p { "Unread messages: {unread}" }
        button { onclick: move |_| unread += 1, "New message" }
        button { onclick: move |_| unread.set(0), "Mark all read" }
    }
}
