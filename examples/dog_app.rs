use dioxus::prelude::*;
use std::collections::HashMap;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut breed = use_signal(|| "deerhound".to_string());
    let mut breed_list = use_future(|| async move {
        let list = reqwest::get("https://dog.ceo/api/breeds/list/all")
            .await
            .unwrap()
            .json::<ListBreeds>()
            .await;

        let Ok(breeds) = list else {
            return render! { "error fetching breeds" };
        };

        render! {
            for cur_breed in breeds.message.keys().take(10).cloned() {
                li { key: "{cur_breed}",
                    button { onclick: move |_| breed.set(cur_breed.clone()),
                        "{cur_breed}"
                    }
                }
            }
        }
    });

    let Some(breed_list) = breed_list.value().cloned() else {
        return render! { "loading breeds..." };
    };

    render! {
        h1 { "Select a dog breed!" }
        div { height: "500px", display: "flex",
            ul { flex: "50%", {breed_list} }
            div { flex: "50%", BreedPic { breed } }
        }
    }
}

#[component]
fn BreedPic(breed: Signal<String>) -> Element {
    let fut = use_future(|| async move {
        reqwest::get(format!("https://dog.ceo/api/breed/{breed}/images/random"))
            .await
            .unwrap()
            .json::<DogApi>()
            .await
    });

    match fut.value().read().as_ref() {
        Some(Ok(resp)) => render! {
            div {
                button { onclick: move |_| fut.restart(), "Click to fetch another doggo" }
                img { max_width: "500px", max_height: "500px", src: "{resp.message}" }
            }
        },
        _ => render! { "loading image..." },
    }
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
struct ListBreeds {
    message: HashMap<String, Vec<String>>,
}

#[derive(serde::Deserialize, Debug)]
struct DogApi {
    message: String,
}
