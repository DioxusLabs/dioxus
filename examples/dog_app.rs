//! Render a bunch of doggos!
//!

use std::collections::HashMap;

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
struct ListBreeds {
    message: HashMap<String, Vec<String>>,
}

fn app(cx: Scope) -> Element {
    let fut = use_future(&cx, || async move {
        reqwest::get("https://dog.ceo/api/breeds/list/all")
            .await
            .unwrap()
            .json::<ListBreeds>()
            .await
    });

    let selected_breed = use_state(&cx, || None);

    match fut.value() {
        Some(Ok(breeds)) => cx.render(rsx! {
            div {
                h1 {"Select a dog breed!"}

                div { display: "flex",
                    ul { flex: "50%",
                        breeds.message.keys().map(|breed| rsx!(
                            li {
                                button {
                                    onclick: move |_| selected_breed.set(Some(breed.clone())),
                                    "{breed}"
                                }
                            }
                        ))
                    }
                    div { flex: "50%",
                        match &*selected_breed {
                            Some(breed) => rsx!( Breed { breed: breed.clone() } ),
                            None => rsx!("No Breed selected"),
                        }
                    }
                }
            }
        }),
        Some(Err(e)) => cx.render(rsx! {
            div { "Error fetching breeds" }
        }),
        None => cx.render(rsx! {
            div { "Loading dogs..." }
        }),
    }
}

#[inline_props]
fn Breed(cx: Scope, breed: String) -> Element {
    #[derive(serde::Deserialize)]
    struct DogApi {
        message: String,
    }

    let endpoint = format!("https://dog.ceo/api/breed/{}/images/random", breed);

    let fut = use_future(&cx, || async move {
        reqwest::get(endpoint).await.unwrap().json::<DogApi>().await
    });

    cx.render(match fut.value() {
        Some(Ok(resp)) => rsx! {
            button {
                onclick: move |_| fut.restart(),
                "Click to fetch another doggo"
            }
            div {
                img {
                    max_width: "500px",
                    max_height: "500px",
                    src: "{resp.message}",
                }
            }
        },
        Some(Err(_)) => rsx! { div { "loading dogs failed" } },
        None => rsx! { div { "loading dogs..." } },
    })
}
