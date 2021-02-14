//! DioxusLabs Webiste
//! ------------------
//!
//! This is the example powering the DioxusLabs website :)
//! It's always good to dogfood your code, right?

use dioxus::prelude::*;

fn main() {}

mod state {
    pub struct AppState {
        cur_page: Route,
    }

    pub enum Route {
        Homepage,
        Docs,
    }
}

static APP: FC<()> = |ctx, props| {
    ctx.view(html! {
        <div>

        <div>
    })
};

/// Draw the navbar on top of the screen
static Navbar: FC<state::Route> = |ctx, props| {
    ctx.view(html! {
        <div>

        <div>
    })
};

static Homepage: FC<()> = |ctx, props| {
    ctx.view(html! {
        <div>

        <div>
    })
};
