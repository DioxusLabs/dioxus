#![allow(non_upper_case_globals)]

//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_web;

fn main() {
    dioxus_web::launch(App);
}

static App: Component = |cx| {
    todo!("suspense is broken")
    // let doggo = suspend(|| async move {
    //     #[derive(serde::Deserialize)]
    //     struct Doggo {
    //         message: String,
    //     }

    //     let src = reqwest::get("https://dog.ceo/api/breeds/image/random")
    //         .await
    //         .expect("Failed to fetch doggo")
    //         .json::<Doggo>()
    //         .await
    //         .expect("Failed to parse doggo")
    //         .message;

    //     rsx!(cx, img { src: "{src}" })
    // });

    // rsx!(cx, div {
    //     h1 {"One doggo coming right up"}
    //     button { onclick: move |_| cx.needs_update(), "Get a new doggo" }
    //     {doggo}
    // })
};
