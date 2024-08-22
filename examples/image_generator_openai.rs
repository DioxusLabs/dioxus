use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Error};

fn main() {
    launch(app)
}

fn app() -> Element {
    let mut loading = use_signal(|| "".to_string());
    let mut api = use_signal(|| "".to_string());
    let mut prompt = use_signal(|| "".to_string());
    let mut n_image = use_signal(|| 1.to_string());
    let mut image = use_signal(|| ImageResponse {
        created: 0,
        data: Vec::new(),
    });

    let mut generate_images = use_resource(move || async move {
        let api_key = api.peek().clone();
        let prompt = prompt.peek().clone();
        let number_of_images = n_image.peek().clone();

        if api_key.is_empty() || prompt.is_empty() || number_of_images.is_empty() {
            return;
        }

        loading.set("is-loading".to_string());

        match request(api_key, prompt, number_of_images).await {
            Ok(imgz) => image.set(imgz),
            Err(e) => println!("Error: {:?}", e),
        }

        loading.set("".to_string());
    });

    rsx! {
        document::Stylesheet { href: "https://unpkg.com/bulma@0.9.0/css/bulma.min.css" }
        div { class: "container",
            div { class: "columns",
                div { class: "column",
                    input { class: "input is-primary mt-4",
                        value: "{api}",
                        r#type: "text",
                        placeholder: "API",
                        oninput: move |evt| api.set(evt.value()),
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
                        value:"{n_image}",
                        oninput: move |evt| n_image.set(evt.value()),
                    }
                }
            }
            button { class: "button is-primary {loading}",
                onclick: move |_| generate_images.restart(),
                "Generate image"
            }
            br {}
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
async fn request(api: String, prompt: String, n_image: String) -> Result<ImageResponse, Error> {
    let client = reqwest::Client::new();
    let body = json!({
        "prompt":  prompt,
        "n":n_image.parse::<i32>().unwrap_or(1),
        "size":"1024x1024",
    });

    let mut authorization = "Bearer ".to_string();
    authorization.push_str(&api);

    let res = client
        .post("https://api.openai.com/v1/images/generations")
        .body(body.to_string())
        .header("Content-Type", "application/json")
        .header("Authorization", authorization)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let deserialized: ImageResponse = serde_json::from_str(&res)?;
    Ok(deserialized)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Props, Clone)]
struct UrlImage {
    url: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Props, Clone)]
struct ImageResponse {
    created: i32,
    data: Vec<UrlImage>,
}
