use dioxus::prelude::*;

use crate::{
    backend::{list_dogs, remove_dog, save_dog},
    Route,
};

#[component]
pub fn Favorites() -> Element {
    let mut favorites = use_loader(list_dogs)?;

    rsx! {
        div { id: "favorites",
            for (id , url) in favorites.cloned() {
                div { class: "favorite-dog", key: "{id}",
                    img { src: "{url}" }
                    button {
                        onclick: move |_| async move {
                            _ = remove_dog(id).await;
                            favorites.restart();
                        },
                        "❌"
                    }
                }
            }
        }
    }
}

#[component]
pub fn NavBar() -> Element {
    rsx! {
        div { id: "title",
            span {}
            Link { to: Route::DogView, h1 { "🌭 HotDog! " } }
            Link { to: Route::Favorites, id: "heart", "♥️" }
        }
        Outlet::<Route> {}
    }
}

#[component]
pub fn DogView() -> Element {
    let mut img_src = use_loader(|| async move {
        anyhow::Ok(
            reqwest::get("https://dog.ceo/api/breeds/image/random")
                .await?
                .json::<serde_json::Value>()
                .await?["message"]
                .to_string(),
        )
    })?;

    rsx! {
        div { id: "dogview",
            img { id: "dogimg", src: "{img_src}" }
        }
        div { id: "buttons",
            button {
                id: "skip",
                onclick: move |_| img_src.restart(),
                "skip"
            }
            button {
                id: "save",
                onclick: move |_| async move { _ = save_dog(img_src()).await },
                "save!"
            }
        }
    }
}
