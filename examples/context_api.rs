//! Demonstrates cross-component state sharing using Dioxus' Context API
//!
//! Features:
//! - Context provider initialization
//! - Nested component consumption
//! - Reactive state updates
//! - Error handling for missing context
//! - Platform-agnostic implementation

use dioxus::prelude::*;

const STYLE: Asset = asset!("/examples/assets/context_api.css");

fn main() {
    launch(app);
}

#[component]
fn app() -> Element {
    // Provide theme context at root level
    use_context_provider(|| Signal::new(Theme::Light));

    rsx! {
        document::Link { rel: "stylesheet", href: STYLE }
        main {
            class: "main-container",

            h1 { "Theme Switcher" }
            ThemeControls {}
            ThemeDisplay {}
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Theme {
    Light,
    Dark,
}

impl Theme {
    fn stylesheet(&self) -> &'static str {
        match self {
            Theme::Light => "light-theme",
            Theme::Dark => "dark-theme",
        }
    }
}

#[component]
fn ThemeControls() -> Element {
    let mut theme = try_use_context::<Signal<Theme>>()
        .expect("Theme context not found. Wrap components in <App>");

    rsx! {
        div {
            class: "controls",
            button {
                class: "btn",
                onclick: move |_| theme.set(Theme::Light),
                disabled: *theme.read() == Theme::Light,
                "Switch to Light"
            }
            button {
                class: "btn",
                onclick: move |_| theme.set(Theme::Dark),
                disabled: *theme.read() == Theme::Dark,
                "Switch to Dark"
            }
        }
    }
}

#[component]
fn ThemeDisplay() -> Element {
    let theme = try_use_context::<Signal<Theme>>()
        .expect("Theme context not found. Wrap components in <App>");

    rsx! {
        div {
            class: "display {theme.read().stylesheet()}",
            p { "Current theme: {theme:?}" }
            p { "Try switching themes using the buttons above!" }
        }
    }
}
