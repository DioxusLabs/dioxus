use dioxus::prelude::*;

#[component]
pub fn DogView() -> Element {
    let mut img_src = use_resource(|| async move {
        #[derive(serde::Deserialize)]
        struct DogApi {
            message: String,
        }

        reqwest::get("https://dog.ceo/api/breeds/image/random")
            .await
            .unwrap()
            .json::<DogApi>()
            .await
            .unwrap()
            .message
    });

    rsx! {
        div { id: "dogview",
            img { id: "dogimg", src: "{img_src().unwrap_or_default()}" }
        }
        div { id: "buttons",
            button {
                id: "skip",
                onclick: move |_| {
                    img_src.restart();
                },
                "skip"
            }
            button {
                id: "save",
                onclick: move |_| async move {
                    img_src.restart();
                    crate::backend::save_dog(img_src().unwrap()).await.unwrap();
                },
                "save!"
            }
        }
    }
}
