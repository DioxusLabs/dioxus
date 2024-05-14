//! Suspense in Dioxus
//!
//! Currently, `rsx!` does not accept futures as values. To achieve the functionality
//! of suspense, we need to make a new component that performs its own suspense
//! handling.
//!
//! In this example, we render the `Doggo` component which starts a future that
//! will cause it to fetch a random dog image from the Dog API. Since the data
//! is not ready immediately, we render some loading text.
//!
//! We can achieve the majority of suspense functionality by composing "suspenseful"
//! primitives in our own custom components.

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

fn main() {
    LaunchBuilder::new()
        .with_cfg(desktop! {
            Config::new().with_window(
                WindowBuilder::new()
                    .with_title("Doggo Fetcher")
                    .with_inner_size(LogicalSize::new(600.0, 800.0)),
            )
        })
        .launch(app)
}

fn app() -> Element {
    rsx! {
        div {
            h1 { "Dogs are very important" }
            p {
                "The dog or domestic dog (Canis familiaris[4][5] or Canis lupus familiaris[5])"
                "is a domesticated descendant of the wolf which is characterized by an upturning tail."
                "The dog derived from an ancient, extinct wolf,[6][7] and the modern grey wolf is the"
                "dog's nearest living relative.[8] The dog was the first species to be domesticated,[9][8]"
                "by hunter–gatherers over 15,000 years ago,[7] before the development of agriculture.[1]"
            }

            h3 { "Illustrious Dog Photo" }
            SuspenseBoundary {
                Doggo {}
                Doggo {}
            }
        }
    }
}

#[component]
fn SuspenseBoundary(children: Element) -> Element {
    let boundary = use_suspense_boundary();
    println!("{:?}", boundary.suspended_futures());
    rsx! {
        div {
            display: if boundary.suspended_futures().is_empty() { "block" } else { "none" },
            {children}
        }
        div {
            display: if boundary.suspended_futures().is_empty() { "none" } else { "block" },
            "loading..."
        }
    }
}

/// This component will re-render when the future has finished
/// Suspense is achieved my moving the future into only the component that
/// actually renders the data.
#[component]
fn Doggo() -> Element {
    println!("Rendering doggo");
    let mut resource = use_resource(move || async move {
        #[derive(serde::Deserialize)]
        struct DogApi {
            message: String,
        }

        reqwest::get("https://dog.ceo/api/breeds/image/random/")
            .await
            .unwrap()
            .json::<DogApi>()
            .await
    });

    // You can suspend the future and only continue rendering when it's ready
    let value = resource.suspend()?;

    match value.read_unchecked().as_ref() {
        Ok(resp) => rsx! {
            button { onclick: move |_| resource.restart(), "Click to fetch another doggo" }
            div { img { max_width: "500px", max_height: "500px", src: "{resp.message}" } }
        },
        Err(_) => rsx! { div { "loading dogs failed" } },
    }
}
