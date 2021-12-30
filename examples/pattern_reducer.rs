//! Example: Reducer Pattern
//! -----------------
//!
//! This example shows how to encapsulate state in dioxus components with the reducer pattern.
//! This pattern is very useful when a single component can handle many types of input that can
//! be represented by an enum.

use dioxus::prelude::*;
fn main() {
    env_logger::init();
    dioxus::desktop::launch(App);
}

pub static App: Component = |cx| {
    let state = use_state(&cx, PlayerState::new);

    cx.render(rsx!(
        div {
            h1 {"Select an option"}
            h3 { "The radio is... " [state.is_playing()], "!" }
            button { onclick: move |_| state.modify().reduce(PlayerAction::Pause),
                "Pause"
            }
            button { onclick: move |_| state.modify().reduce(PlayerAction::Play),
                "Play"
            }
        }
    ))
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
