//! Example: Reducer Pattern
//! -----------------
//!
//! This example shows how to encapsulate state in dioxus components with the reducer pattern.
//! This pattern is very useful when a single component can handle many types of input that can
//! be represented by an enum.

use dioxus::prelude::*;

const STYLE: Asset = asset!("/examples/assets/radio.css");

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut state = use_signal(|| PlayerState { is_playing: false });

    rsx!(
        document::Stylesheet { href: STYLE }
        h1 {"Select an option"}

        // Add some cute animations if the radio is playing!
        div { class: if state.read().is_playing { "bounce" },
            "The radio is... " {state.read().is_playing()} "!"
        }

        button { id: "play", onclick: move |_| state.write().reduce(PlayerAction::Pause), "Pause" }
        button { id: "pause", onclick: move |_| state.write().reduce(PlayerAction::Play), "Play" }
    )
}

enum PlayerAction {
    Pause,
    Play,
}

#[derive(Clone)]
struct PlayerState {
    is_playing: bool,
}

impl PlayerState {
    fn reduce(&mut self, action: PlayerAction) {
        match action {
            PlayerAction::Pause => self.is_playing = false,
            PlayerAction::Play => self.is_playing = true,
        }
    }
    fn is_playing(&self) -> &'static str {
        match self.is_playing {
            true => "currently playing!",
            false => "not currently playing",
        }
    }
}
