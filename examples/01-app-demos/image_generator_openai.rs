use dioxus::prelude::*;

fn main() {
    dioxus::launch(app)
}

fn app() -> Element {
    let mut api_key = use_signal(|| "".to_string());
    let mut prompt = use_signal(|| "".to_string());
    let mut num_images = use_signal(|| 1.to_string());

    let mut image = use_action(move || async move {
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Props, Clone, Default)]
        struct ImageResponse {
            created: i32,
            data: Vec<UrlImage>,
        }

        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Props, Clone)]
        struct UrlImage {
            url: String,
        }

        if api_key.peek().is_empty() || prompt.peek().is_empty() || num_images.peek().is_empty() {
            return dioxus::Ok(ImageResponse::default());
        }

        let res = reqwest::Client::new()
            .post("https://api.openai.com/v1/images/generations")
            .json(&serde_json::json!({
                "prompt":  prompt.cloned(),
                "n": num_images.cloned().parse::<i32>().unwrap_or(1),
                "size":"1024x1024",
            }))
            .bearer_auth(api_key)
            .send()
            .await?
            .json::<ImageResponse>()
            .await?;

        Ok(res)
    });

    rsx! {
        Stylesheet { href: "https://unpkg.com/bulma@0.9.0/css/bulma.min.css" }
        div { class: "container",
            div { class: "columns",
                div { class: "column",
                    input { class: "input is-primary mt-4",
                        value: "{api_key}",
                        r#type: "text",
                        placeholder: "Your OpenAI API Key",
                        oninput: move |evt| api_key.set(evt.value()),
                    }
                    input { class: "input is-primary mt-4",
                        placeholder: "MAX 1000 Dgts",
                        r#type: "text",
                        value:"{prompt}",
                        oninput: move |evt| prompt.set(evt.value())
                    }
                    input { class: "input is-primary mt-4",
                        r#type: "number",
                        min:"1",
                        max:"10",
                        value:"{num_images}",
                        oninput: move |evt| num_images.set(evt.value()),
                    }
                }
            }
            button {
                class: "button is-primary",
                class: if image.pending() { "is-loading" },
                onclick: move |_| {
                    image.call();
                },
                "Generate image"
            }
            if let Some(Ok(image)) = image.value() {
                for image in image.read().data.as_slice() {
                    section { class: "is-flex",
                        div { class: "container is-fluid",
                            div { class: "container has-text-centered",
                                div { class: "is-justify-content-center",
                                    div { class: "level",
                                        div { class: "level-item",
                                            figure { class: "image", img { alt: "", src: "{image.url}", } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
