//! This example shows how to use the hash segment to store state in the url.
//!
//! You can set up two way data binding between the url hash and signals.

use std::{fmt::Display, str::FromStr};

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

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
    #[route("/#:url_hash")]
    Home {
        url_hash: State,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
struct State {
    counters: Vec<usize>,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

impl FromHashFragment for State {
    fn from_hash_fragment(hash: &str) -> Self {
        serde_json::from_str(hash).unwrap_or_default()
    }
}

#[component]
fn Home(url_hash: ReadOnlySignal<State>) -> Element {
    // The initial state of the state comes from the url hash
    let mut state = use_signal(&*url_hash);

    // Change the state signal when the url hash changes
    use_memo(move || {
        if *state.peek() != *url_hash.read() {
            state.set(url_hash());
        }
    });

    // Change the url hash when the state changes
    use_memo(move || {
        if *state.read() != *url_hash.peek() {
            navigator().replace(Route::Home { url_hash: state() });
        }
    });

    rsx! {
        button {
            onclick: move |_| state.write().counters.clear(),
            "Reset"
        }
        button {
            onclick: move |_| {
                state.write().counters.push(0);
            },
            "Add Counter"
        }
        for counter in 0..state.read().counters.len() {
            div {
                button {
                    onclick: move |_| {
                        state.write().counters.remove(counter);
                    },
                    "Remove"
                }
                button {
                    onclick: move |_| {
                        state.write().counters[counter] += 1;
                    },
                    "Count: {state.read().counters[counter]}"
                }
            }
        }
    }
}
