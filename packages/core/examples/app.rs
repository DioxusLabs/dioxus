/*
This example shows how to encapsulate sate in dioxus components with the reducer pattern.
This pattern is very useful when a single component can handle many types of input that can
be represented by an enum. This particular pattern is very powerful in rust where ADTs can simplify
much of the traditional reducer boilerplate.
*/
#![allow(unused)]
use std::future::Future;

use dioxus::hooks::use_reducer;
use dioxus_ssr::prelude::*;

enum Actions {
    Pause,
    Play,
}

struct SomeState {
    is_playing: bool,
}

impl SomeState {
    fn new() -> Self {
        Self { is_playing: false }
    }
    fn reduce(&mut self, action: Actions) {
        match action {
            Actions::Pause => self.is_playing = false,
            Actions::Play => self.is_playing = true,
        }
    }
    fn is_playing(&self) -> &'static str {
        match self.is_playing {
            true => "currently playing!",
            false => "not currently playing",
        }
    }
}

pub static ExampleReducer: FC<()> = |ctx| {
    let (state, reduce) = use_reducer(&ctx, SomeState::new, SomeState::reduce);

    let is_playing = state.is_playing();

    ctx.render(rsx! {
        div {
            h1 {"Select an option"}
            h3 {"The radio is... {is_playing}!"}
            button {
                "Pause"
                onclick: move |_| reduce(Actions::Pause)
            }
            button {
                "Play"
                onclick: move |_| reduce(Actions::Play)
            }
        }
    })
};

/*














*/

struct AppContext {
    name: String,
}

enum KindaState {
    Ready,
    Complete,
    Erred,
}

static EnumReducer: FC<()> = |ctx| {
    let (state, reduce) = use_reducer(&ctx, || KindaState::Ready, |cur, new| *cur = new);

    let contents = helper(&ctx);

    let status = match state {
        KindaState::Ready => "Ready",
        KindaState::Complete => "Complete",
        KindaState::Erred => "Erred",
    };

    ctx.render(rsx! {
        div {
            h1 {"{status}"}
            {contents}
            button {
                "Set Ready"
                onclick: move |_| reduce(KindaState::Ready)
            }
            button {
                "Set Complete"
                onclick: move |_| reduce(KindaState::Complete)
            }
            button {
                "Set Erred"
                onclick: move |_| reduce(KindaState::Erred)
            }
            ul {
                {(0..10).map(|f| {

                    rsx!{
                        li {
                            "hello there!"
                        }
                    }
                })}
            }
        }
    })
};

fn helper(ctx: &Context) -> VNode {
    ctx.render(rsx! {
        div {}
    })
}

/// Demonstrate how the DebugRenderer can be used to unit test components without needing a browser
/// These tests can run locally.
/// They use the "compare" method of the debug renderer to do partial tree compares for succint
#[test]
fn ensure_it_works_properly() -> dioxus::error::Result<()> {
    let mut test = DebugRenderer::new(EnumReducer);
    test.compare(rsx! { div { h1 {"Ready"} } })?;

    test.trigger_listener(1)?;
    test.compare(rsx! { div { h1 {"Ready"} } })?;

    test.trigger_listener(2)?;
    test.compare(rsx! { div { h1 {"Complete"} } })?;

    test.trigger_listener(3)?;
    test.compare(rsx! { div { h1 {"Erred"} } })?;
    Ok(())
}
