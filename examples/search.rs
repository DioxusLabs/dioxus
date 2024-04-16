//! A search example that integrates with the router for sharable search results.
//!
//! The enum router makes it easy to use your route as state in your app. This example shows how to use the router to encode search text into the url and decode it back into a string.

use dioxus::prelude::*;

fn main() {
    launch(|| {
        rsx! {
            Router::<Route> {}
        }
    });
}

#[derive(Routable, Clone, Debug, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    Home {},

    #[route("/search?:query")] 
    Search {
        query: String,
    },
}

#[component]
fn Home() -> Element {
    rsx! {
        Link {
            to: Route::Search {
                query: "hello".to_string()
            },
            "Search for hello"
        }
        Link {
            to: Route::Search {
                query: "dioxus".to_string()
            },
            "Search for dioxus"
        }
    }
}

#[component]
fn Search(query: ReadOnlySignal<String>) -> Element {
    const ITEMS: &[&str] = &["hello world", "hello dioxus", "hello dioxus-rsx"];

    let results = use_memo(move || {
        ITEMS
            .iter()
            .filter(|item| item.contains(&*query.read()))
            .collect::<Vec<_>>()
    });

    rsx! {
        h1 { "Search for {query}" }
        input {
            oninput: move |e| {
                navigator().push(Route::Search {
                    query: e.value(),
                });
            },
            value: "{query}",
        }
        for result in results.read().iter() {
            div { "{result}" }
        }
    }
}
