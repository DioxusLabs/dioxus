//! Chaining Resources with `read_async()`

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app)
}

fn app() -> Element {
    let mut image_url = use_signal(|| "Loading...".to_owned());
    let mut breed_info = use_signal(|| "Waiting for dog image...".to_owned());
    let mut request_count = use_signal(|| 0);
    let mut analysis_result = use_signal(|| "No analysis yet".to_owned());

    #[derive(serde::Deserialize, Clone)]
    struct DogImageResponse {
        message: String,
    }

    let mut dog_image = use_resource(move || async move {
        reqwest::get("https://dog.ceo/api/breeds/image/random")
            .await?
            .json::<DogImageResponse>()
            .await
    });

    let process_image = use_resource(move || async move {
        // Wait for result from other resource
        let result_ref = dog_image.read_async().await;

        // Clone the data we need from the guard
        let url = match &*result_ref {
            Ok(response) => response.message.clone(),
            Err(e) => return Err(anyhow::anyhow!("Failed to fetch image: {}", e)),
        };

        // Drop the guard before doing more async work
        drop(result_ref);

        // Simulate some async processing (e.g., image validation, metadata fetch)
        document::eval(r#"await new Promise(resolve => setTimeout(resolve, 500)); return null;"#)
            .await
            .unwrap();

        // Now we can safely do a sync read since we know the value exists
        let result = dog_image.read();
        let response = result.as_ref().unwrap().as_ref().unwrap();

        let mut image_url_write = image_url.write();
        let mut breed_info_write = breed_info.write();
        let mut count_write = request_count.write();

        *image_url_write = url.clone();

        let breed = if let Some(breed_part) = response.message.split("/breeds/").nth(1) {
            if let Some(breed) = breed_part.split('/').next() {
                breed.replace('-', " ").to_uppercase()
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };

        *breed_info_write = format!("Breed: {}", breed);
        *count_write += 1;

        Ok::<String, anyhow::Error>(breed)
    });

    // Second resource: Uses the breed from the first resource to fetch additional info
    let _breed_analyzer = use_resource(move || async move {
        // Wait for the process_image resource to complete
        let breed_result = process_image.read_async().await;

        let breed = match &*breed_result {
            Ok(breed) => breed.clone(),
            Err(e) => {
                let mut analysis = analysis_result.write();
                *analysis = format!("Analysis failed: {}", e);
                return Err(anyhow::anyhow!("No breed to analyze"));
            }
        };

        // Drop before async work
        drop(breed_result);

        // Simulate fetching additional breed information
        document::eval(r#"await new Promise(resolve => setTimeout(resolve, 500)); return null;"#)
            .await
            .unwrap();

        let analysis = format!(
            "Analysis: {} breed detected. Image URL length: {} chars. Fetch timestamp: {}",
            breed,
            image_url.read().len(),
            request_count.read()
        );

        let mut analysis_write = analysis_result.write();
        *analysis_write = analysis.clone();

        Ok::<String, anyhow::Error>(analysis)
    });

    rsx! {
        div { style: "font-family: Arial, sans-serif; max-width: 600px; margin: 50px auto; text-align: center;",

            h1 { "🐕 Random Dog Image Fetcher" }

            p { style: "color: #666; margin-bottom: 20px;",
                "This example demonstrates chaining resources with "
                code { "read_async()" }
                " to safely handle guards across await points."
            }

            div { style: "background: #f5f5f5; padding: 20px; border-radius: 8px; margin: 20px 0;",

                match &*dog_image.value().read() {
                    Some(Ok(_)) => rsx! {
                        img {
                            src: "{image_url}",
                            alt: "Random dog",
                            style: "max-width: 100%; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1);",
                        }
                    },
                    Some(Err(e)) => rsx! {
                        p { style: "color: red;", "❌ Error: {e}" }
                    },
                    None => rsx! {
                        p { "⏳ Loading dog image..." }
                    },
                }

                p { style: "margin-top: 15px; font-size: 18px; font-weight: bold;",
                    "{breed_info}"
                }

                p { style: "color: #888; font-size: 14px; margin-top: 10px;",
                    "{analysis_result}"
                }

                p { style: "color: #888; font-size: 14px;", "Images fetched: {request_count}" }
            }

            div { style: "display: flex; gap: 10px; justify-content: center; margin: 20px 0;",
                button {
                    onclick: move |_| dog_image.restart(),
                    style: "background: #4CAF50; color: white; border: none; padding: 12px 24px; font-size: 16px; border-radius: 4px; cursor: pointer;",
                    "🔄 Fetch Another Dog"
                }

                button {
                    onclick: move |_| async move {
                        // This demonstrates using read_async in an onclick callback
                        let breed_result = process_image.read_async().await;
                        if let Ok(breed) = &*breed_result {
                            let mut analysis = analysis_result.write();
                            *analysis = format!("Manual trigger: Analyzing {} breed...", breed);
                        }
                    },
                    style: "background: #2196F3; color: white; border: none; padding: 12px 24px; font-size: 16px; border-radius: 4px; cursor: pointer;",
                    "🔍 Re-analyze Breed"
                }
            }
        }
    }
}
