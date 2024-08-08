//! This example shows how to use the hash segment to store state in the url.
//!
//! You can set up two way data binding between the url hash and signals.
//!
//! Run this example on desktop with  
//! ```sh
//! dx serve --example hash_fragment_state --features=ciborium,base64
//! ```
//! Or on web with
//! ```sh
//! dx serve --platform web --features web --example hash_fragment_state --features=ciborium,base64 -- --no-default-features
//! ```

use std::{fmt::Display, str::FromStr};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
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

// You can use a custom type with the hash segment as long as it implements Display, FromStr and Default
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
struct State {
    counters: Vec<usize>,
}

// Display the state in a way that can be parsed by FromStr
impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut serialized = Vec::new();
        if ciborium::into_writer(self, &mut serialized).is_ok() {
            write!(f, "{}", STANDARD.encode(serialized))?;
        }
        Ok(())
    }
}

enum StateParseError {
    DecodeError(base64::DecodeError),
    CiboriumError(ciborium::de::Error<std::io::Error>),
}

impl std::fmt::Display for StateParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DecodeError(err) => write!(f, "Failed to decode base64: {}", err),
            Self::CiboriumError(err) => write!(f, "Failed to deserialize: {}", err),
        }
    }
}

// Parse the state from a string that was created by Display
impl FromStr for State {
    type Err = StateParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let decompressed = STANDARD
            .decode(s.as_bytes())
            .map_err(StateParseError::DecodeError)?;
        let parsed = ciborium::from_reader(std::io::Cursor::new(decompressed))
            .map_err(StateParseError::CiboriumError)?;
        Ok(parsed)
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
