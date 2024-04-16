//! This example shows how to access and use query segments present in an url on the web.
//!
//! The enum router makes it easy to use your route as state in your app. This example shows how to use the router to encode search text into the url and decode it back into a string.
//!
//! Run this example on desktop with  
//! ```sh
//! dx serve --example query_segment_search
//! ```
//! Or on web with
//! ```sh
//! dx serve --platform web --features web --example query_segment_search -- --no-default-features
//! ```

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

    // The each query segment must implement <https://docs.rs/dioxus-router/latest/dioxus_router/routable/trait.FromQueryArgument.html> and Display.
    // You can use multiple query segments separated by `&`s.
    #[route("/search?:query&:word_count")] 
    Search {
        query: String,
        word_count: usize,
    },
}

#[component]
fn Home() -> Element {
    // Display a list of example searches in the home page
    rsx! {
        ul {
            li {
                Link {
                    to: Route::Search {
                        query: "hello".to_string(),
                        word_count: 1
                    },
                    "Search for results containing 'hello' and at least one word"
                }
            }
            li {
                Link {
                    to: Route::Search {
                        query: "dioxus".to_string(),
                        word_count: 2
                    },
                    "Search for results containing 'dioxus' and at least two word"
                }
            }
        }
    }
}

// Instead of accepting String and usize directly, we use ReadOnlySignal to make the parameters `Copy` and let us subscribe to them automatically inside the meme
#[component]
fn Search(query: ReadOnlySignal<String>, word_count: ReadOnlySignal<usize>) -> Element {
    const ITEMS: &[&str] = &[
        "hello",
        "world",
        "hello world",
        "hello dioxus",
        "hello dioxus-router",
    ];

    // Find all results that contain the query and the right number of words
    // This memo will automatically rerun when the query or word count changes because we read the signals inside the closure
    let results = use_memo(move || {
        ITEMS
            .iter()
            .filter(|item| {
                item.contains(&*query.read()) && item.split_whitespace().count() >= word_count()
            })
            .collect::<Vec<_>>()
    });

    rsx! {
        h1 { "Search for {query}" }
        input {
            oninput: move |e| {
                // Every time the query changes, we change the current route to the new query
                navigator().replace(Route::Search {
                    query: e.value(),
                    word_count: word_count(),
                });
            },
            value: "{query}",
        }
        input {
            r#type: "number",
            oninput: move |e| {
                // Every time the word count changes, we change the current route to the new query
                if let Ok(word_count) = e.value().parse() {
                    navigator().replace(Route::Search {
                        query: query(),
                        word_count,
                    });
                }
            },
            value: "{word_count}",
        }
        for result in results.read().iter() {
            div { "{result}" }
        }
    }
}
