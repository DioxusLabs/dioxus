//! Example: Reducer Pattern
//! -----------------
//!
//! This example shows how to encapsulate state in dioxus components with the reducer pattern.
//! This pattern is very useful when a single component can handle many types of input that can
//! be represented by an enum.

use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut state = use_signal(|| PlayerState { is_playing: false });

    render!(
        div {
            h1 {"Select an option"}
            h3 { "The radio is... ", {state.read().is_playing()}, "!" }
            button { onclick: move |_| state.write().reduce(PlayerAction::Pause), "Pause" }
            button { onclick: move |_| state.write().reduce(PlayerAction::Play), "Play" }
        }
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
