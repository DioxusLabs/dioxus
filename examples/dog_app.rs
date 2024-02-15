//! This example demonstrates a simple app that fetches a list of dog breeds and displays a random dog.
//!
//! The app uses the `use_signal` and `use_resource` hooks to manage state and fetch data from the Dog API.
//! `use_resource` is basically an async version of use_memo - it will track dependencies between .await points
//! and then restart the future if any of the dependencies change.
//!
//! You should generally throttle requests to an API - either client side or server side. This example doesn't do that
//! since it's unlikely the user will rapidly cause new fetches, but it's something to keep in mind.

use dioxus::prelude::*;
use std::collections::HashMap;

fn main() {
    launch(app);
}

fn app() -> Element {
    // Breed is a signal that will be updated when the user clicks a breed in the list
    // `deerhound` is just a default that we know will exist. We could also use a `None` instead
    let mut breed = use_signal(|| "deerhound".to_string());

    // Fetch the list of breeds from the Dog API
    // Since there are no dependencies, this will never restart
    let breed_list = use_resource(move || async move {
        #[derive(Debug, Clone, PartialEq, serde::Deserialize)]
        struct ListBreeds {
            message: HashMap<String, Vec<String>>,
        }

        let list = reqwest::get("https://dog.ceo/api/breeds/list/all")
            .await
            .unwrap()
            .json::<ListBreeds>()
            .await;

        let Ok(breeds) = list else {
            return rsx! { "error fetching breeds" };
        };

        rsx! {
            for cur_breed in breeds.message.keys().take(20).cloned() {
                li { key: "{cur_breed}",
                    button { onclick: move |_| breed.set(cur_breed.clone()),
                        "{cur_breed}"
                    }
                }
            }
        }
    });

    // We can use early returns in dioxus!
    // Traditional signal-based libraries can't do this since the scope is by default non-reactive
    let Some(breed_list) = breed_list() else {
        return rsx! { "loading breeds..." };
    };

    rsx! {
        style { {include_str!("./assets/dog_app.css")} }
        h1 { "Select a dog breed!" }
        div { height: "500px", display: "flex",
            ul { width: "100px", {breed_list} }
            div { flex: 1, BreedPic { breed } }
        }
    }
}

#[component]
fn BreedPic(breed: Signal<String>) -> Element {
    // This resource will restart whenever the breed changes
    let mut fut = use_resource(move || async move {
        #[derive(serde::Deserialize, Debug)]
        struct DogApi {
            message: String,
        }

        reqwest::get(format!("https://dog.ceo/api/breed/{breed}/images/random"))
            .await
            .unwrap()
            .json::<DogApi>()
            .await
    });

    match fut.read().as_ref() {
        Some(Ok(resp)) => rsx! {
            button { onclick: move |_| fut.restart(), "Click to fetch another doggo" }
            img { max_width: "500px", max_height: "500px", src: "{resp.message}" }
        },
        Some(Err(_)) => rsx! { "loading image failed" },
        None => rsx! { "loading image..." },
    }
}
