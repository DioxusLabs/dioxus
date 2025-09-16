//! This example demonstrates a simple app that fetches a list of dog breeds and displays a random dog.
//!
//! This app combines `use_loader` and `use_action` to fetch data from the Dog API.
//! - `use_loader` automatically fetches the list of dog breeds when the component mounts.
//! - `use_action` fetches a random dog image whenever the `.dispatch` method is called.

use dioxus::prelude::*;
use std::collections::HashMap;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // Fetch the list of breeds from the Dog API, using the `?` syntax to suspend or throw errors
    let breed_list = use_loader(move || async move {
        #[derive(serde::Deserialize, Debug, PartialEq, Clone)]
        struct ListBreeds {
            message: HashMap<String, Vec<String>>,
        }

        reqwest::get("https://dog.ceo/api/breeds/list/all")
            .await?
            .json::<ListBreeds>()
            .await
    })?;

    // Whenever this action is called, it will re-run the future and return the result.
    let mut breed = use_action(move |breed| async move {
        #[derive(serde::Deserialize, Debug, PartialEq, Clone)]
        struct DogApi {
            message: String,
        }

        reqwest::get(format!("https://dog.ceo/api/breed/{breed}/images/random"))
            .await
            .unwrap()
            .json::<DogApi>()
            .await
    });

    rsx! {
        h1 { "Doggo selector" }
        div { width: "400px",
            for cur_breed in breed_list.read().message.keys().take(20).cloned() {
                button {
                    onclick: move |_| {
                        breed.dispatch(cur_breed.clone());
                    },
                    "{cur_breed}"
                }
            }
        }
        div {
            match breed.result().map(|res| res.cloned()) {
                Err(_e) => rsx! { div { "Failed to fetch a dog, please try again." } },
                Ok(None) => rsx! { div { "Click the button to fetch a dog!" } },
                Ok(Some(res)) => rsx! {
                    img {
                        max_width: "500px",
                        max_height: "500px",
                        src: "{res.message}"
                    }
                },
            }
        }
    }
}
