//! Example: Reducer Pattern
//! -----------------
//! This example shows how to encapsulate sate in dioxus components with the reducer pattern.
//! This pattern is very useful when a single component can handle many types of input that can
//! be represented by an enum.

fn main() {}
use dioxus::prelude::*;

pub static ExampleReducer: FC<()> = |ctx| {
    let (state, reduce) = use_reducer(&ctx, PlayerState::new, PlayerState::reduce);

    let is_playing = state.is_playing();

    ctx.render(rsx! {
        div {
            h1 {"Select an option"}
            h3 {"The radio is... {is_playing}!"}
            button {
                "Pause"
                onclick: move |_| reduce(PlayerAction::Pause)
            }
            button {
                "Play"
                onclick: move |_| reduce(PlayerAction::Play)
            }
        }
    })
};

enum PlayerAction {
    Pause,
    Play,
}

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
