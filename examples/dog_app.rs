#![allow(non_snake_case)]

//! Render a bunch of doggos!

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
    let breed = use_state(&cx, || None);

    let breeds = use_future(&cx, (), |_| async move {
        reqwest::get("https://dog.ceo/api/breeds/list/all")
            .await
            .unwrap()
            .json::<ListBreeds>()
            .await
    });

    match breeds.value() {
        Some(Ok(breeds)) => cx.render(rsx! {
            div {
                h1 { "Select a dog breed!" }
                div { display: "flex",
                    ul { flex: "50%",
                        breeds.message.keys().map(|cur_breed| rsx!(
                            li {
                                button {
                                    onclick: move |_| breed.set(Some(cur_breed.clone())),
                                    "{cur_breed}"
                                }
                            }
                        ))
                    }
                    div { flex: "50%",
                        match breed.get() {
                            Some(breed) => rsx!( Breed { breed: breed.clone() } ),
                            None => rsx!("No Breed selected"),
                        }
                    }
                }
            }
        }),
        Some(Err(_e)) => cx.render(rsx! { div { "Error fetching breeds" } }),
        None => cx.render(rsx! { div { "Loading dogs..." } }),
    }
}

#[derive(serde::Deserialize, Debug)]
struct DogApi {
    message: String,
}

#[inline_props]
fn Breed(cx: Scope, breed: String) -> Element {
    let fut = use_future(&cx, (breed,), |(breed,)| async move {
        let endpoint = format!("https://dog.ceo/api/breed/{}/images/random", breed);
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
