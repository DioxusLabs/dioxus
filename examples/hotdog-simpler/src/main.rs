use dioxus::prelude::*;

mod backend;
use backend::{list_dogs, remove_dog, save_dog};

fn main() {
    #[cfg(not(feature = "server"))]
    server_fn::client::set_server_url("https://hot-dog.fly.dev");

    dioxus::launch(|| {
        rsx! {
            Stylesheet { href: asset!("/assets/main.css") }
            div { id: "title",
                Link { to: "/", h1 { "🌭 HotDog! " } }
                Link { to: "/favroites", id: "heart", "♥️" }
            }
            Route { to: "/", DogView { } }
            Route { to: "/", Favorites { } }
        }
    });
}

#[component]
pub fn Favorites() -> Element {
    let mut favorites = use_loader(list_dogs)?;

    rsx! {
        div { id: "favorites",
            for (id, url) in favorites.cloned() {
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
pub fn DogView() -> Element {
    let mut img_src = use_loader(|| async move {
        Ok(reqwest::get("https://dog.ceo/api/breeds/image/random")
            .await?
            .json::<serde_json::Value>()
            .await?["message"]
            .to_string())
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
