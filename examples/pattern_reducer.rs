//! Example: Reducer Pattern
//! -----------------
//!
//! This example shows how to encapsulate state in dioxus components with the reducer pattern.
//! This pattern is very useful when a single component can handle many types of input that can
//! be represented by an enum.

use dioxus::prelude::*;
fn main() {
    env_logger::init();
    dioxus::desktop::launch(App, |c| c);
}

pub static App: FC<()> = |cx, _| {
    let state = use_state(cx, PlayerState::new);

    let is_playing = state.is_playing();

    rsx!(cx, div {
        h1 {"Select an option"}
        h3 {"The radio is... {is_playing}!"}
        button {
            "Pause"
            onclick: move |_| state.modify().reduce(PlayerAction::Pause)
        }
        button {
            "Play"
            onclick: move |_| state.modify().reduce(PlayerAction::Play)
        }
    })
};

enum PlayerAction {
    Pause,
    Play,
}

#[derive(Clone)]
struct PlayerState {
    is_playing: bool,
}

impl PlayerState {
    fn new() -> Self {
        Self { is_playing: false }
    }
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
