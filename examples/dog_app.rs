#![allow(non_snake_case)]

//! Render a bunch of doggos!

use dioxus::prelude::*;
use std::collections::HashMap;

fn main() {
    dioxus_desktop::launch(|cx| {
        cx.render(rsx! {
            app_root {}
        })
    });
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
struct ListBreeds {
    message: HashMap<String, Vec<String>>,
}

async fn app_root(cx: Scope<'_>) -> Element {
    let breed = use_state(&cx, || "deerhound".to_string());
    let breeds = use_future(&cx, (), |_| async move {
        reqwest::get("https://dog.ceo/api/breeds/list/all")
            .await
            .unwrap()
            .json::<ListBreeds>()
            .await
    });

    match breeds.await {
        Ok(breeds) => cx.render(rsx! {
            div { height: "500px",
                h1 { "Select a dog breed!" }
                div { display: "flex",
                    ul { flex: "50%",
                        for cur_breed in breeds.message.keys().take(10) {
                            rsx! {
                                li { key: "{cur_breed}",
                                    button {
                                        onclick: move |_| breed.set(cur_breed.clone()),
                                        "{cur_breed}"
                                    }
                                }
                            }
                        }
                    }
                    div { flex: "50%", Breed { breed: breed.to_string() } }
                }
            }
        }),
        Err(_e) => cx.render(rsx! { div { "Error fetching breeds" } }),
    }
}

#[derive(serde::Deserialize, Debug)]
struct DogApi {
    message: String,
}

#[inline_props]
async fn Breed(cx: Scope, breed: String) -> Element {
    println!("Rendering Breed: {}", breed);

    let fut = use_future(&cx, (breed,), |(breed,)| async move {
        reqwest::get(format!("https://dog.ceo/api/breed/{}/images/random", breed))
            .await
            .unwrap()
            .json::<DogApi>()
            .await
    });

    let resp = fut.await;

    println!("achieved results!");

    match resp {
        Ok(resp) => cx.render(rsx! {
            div {
                button {
                    onclick: move |_| fut.restart(),
                    "Click to fetch another doggo"
                }
                img {
                    src: "{resp.message}",
                    max_width: "500px",
                    max_height: "500px",
                }
            }
        }),
        Err(e) => cx.render(rsx! { div { "loading dogs failed" } }),
    }
}
