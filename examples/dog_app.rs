use dioxus::prelude::*;
use std::collections::HashMap;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    let breed = use_signal(|| "deerhound".to_string());

    let breed_list = use_future(|| async move {
        reqwest::get("https://dog.ceo/api/breeds/list/all")
            .await
            .unwrap()
            .json::<ListBreeds>()
            .await
    });

    match breed_list.value().read().as_ref() {
        Some(Ok(breeds)) => rsx! {
            div { height: "500px",
                h1 { "Select a dog breed!" }
                div { display: "flex",
                    ul { flex: "50%",
                        for cur_breed in breeds.message.keys().take(10).cloned() {
                            li { key: "{cur_breed}",
                                button { onclick: move |_| breed.set(cur_breed.clone()), "{cur_breed}" }
                            }
                        }
                    }
                    div { flex: "50%", BreedPic { breed } }
                }
            }
        },
        _ => rsx! { div { "loading breeds" } },
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
        Some(Ok(resp)) => rsx! {
            div {
                button { onclick: move |_| fut.restart(), "Click to fetch another doggo" }
                img { max_width: "500px", max_height: "500px", src: "{resp.message}" }
            }
        },
        _ => rsx! { div { "loading dog picture" } },
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
