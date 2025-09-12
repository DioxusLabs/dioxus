//! Suspense in Dioxus
//!
//! Suspense allows components to bubble up loading states to parent components, simplifying data fetching.

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

fn main() {
    dioxus::LaunchBuilder::new()
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
            ErrorBoundary { handle_error: |_| rsx! { p { "Error loading doggos" } },
                SuspenseBoundary { fallback: move |_| rsx! { "Loading doggos..." },
                    Doggo {}
                }
            }
        }
    }
}

#[component]
fn Doggo() -> Element {
    // `use_loader` returns a Result<Loader<T>, LoaderError>. LoaderError can either be "Pending" or "Failed".
    // When we use the `?` operator, the pending/error state will be thrown to the nearest Suspense or Error boundary.
    let mut dog = use_loader(move || async move {
        #[derive(serde::Deserialize, PartialEq)]
        struct DogApi {
            message: String,
        }

        reqwest::get("https://dog.ceo/api/breeds/image/random/")
            .await
            .unwrap()
            .json::<DogApi>()
            .await
    })?;

    rsx! {
        button { onclick: move |_| dog.restart(), "Click to fetch another doggo" }
        div {
            img {
                max_width: "500px",
                max_height: "500px",
                src: "{dog.read().message}"
            }
        }
    }
}
