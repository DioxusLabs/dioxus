//! Chaining Resources with `read_async()`

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app)
}

fn app() -> Element {
    let mut breed_info = use_signal(|| "Waiting for dog image...".to_owned());
    let mut request_count = use_signal(|| 0);
    let mut analysis_result = use_signal(|| "No analysis yet".to_owned());

    #[derive(serde::Deserialize, Clone)]
    struct DogImageResponse {
        message: String,
    }

    // Base resource: fetches the dog image
    let mut dog_image = use_resource(move || async move {
        reqwest::get("https://dog.ceo/api/breeds/image/random")
            .await?
            .json::<DogImageResponse>()
            .await
    });

    // Second resource: extracts breed info from the base image
    let process_image = use_resource(move || async move {
        // Wait for result from base resource
        let result_ref = dog_image.read_async().await;

        // Clone the URL we need from the guard
        let url = match &*result_ref {
            Ok(response) => response.message.clone(),
            Err(e) => return Err(anyhow::anyhow!("Failed to fetch image: {}", e)),
        };

        // Drop the guard before doing more async work
        drop(result_ref);

        // Simulate some async processing (e.g., image validation, metadata fetch)
        document::eval(r#"await new Promise(resolve => setTimeout(resolve, 2000)); return null;"#)
            .await
            .unwrap();

        // Extract breed from URL
        let breed = if let Some(breed_part) = url.split("/breeds/").nth(1) {
            if let Some(breed) = breed_part.split('/').next() {
                breed.replace('-', " ").to_uppercase()
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };

        let mut breed_info_write = breed_info.write();
        let mut count_write = request_count.write();

        *breed_info_write = format!("Breed: {}", breed);
        *count_write += 1;

        Ok::<String, anyhow::Error>(breed)
    });

    // Third resource: analyzes the base image data independently
    let _breed_analyzer = use_resource(move || async move {
        // This also depends on the base dog_image resource
        let image_result = dog_image.read_async().await;

        let url = match &*image_result {
            Ok(response) => response.message.clone(),
            Err(e) => {
                let mut analysis = analysis_result.write();
                *analysis = format!("Analysis failed: {}", e);
                return Err(anyhow::anyhow!("No image to analyze"));
            }
        };

        // Drop before async work
        drop(image_result);

        // Simulate fetching additional information
        document::eval(r#"await new Promise(resolve => setTimeout(resolve, 300)); return null;"#)
            .await
            .unwrap();

        let analysis = format!(
            "Analysis: Image URL length: {} chars. Path segments: {}. Request #{}",
            url.len(),
            url.split('/').count(),
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
                "."
            }

            div { style: "background: #f5f5f5; padding: 20px; border-radius: 8px; margin: 20px 0;",

                match &*dog_image.value().read() {
                    Some(Ok(response)) => rsx! {
                        img {
                            src: "{response.message}",
                            alt: "Random dog",
                            style: "width: 400px; height: 400px; object-fit: cover; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1);",
                        }
                    },
                    Some(Err(e)) => rsx! {
                        div { style: "width: 400px; height: 400px;",
                            p { style: "color: red;", "❌ Error: {e}" }
                        }
                    },
                    None => rsx! {
                        div { style: "width: 400px; height: 400px;",
                            p { "⏳ Loading dog image..." }
                        }
                    },
                }

                p { style: "margin-top: 15px; font-size: 18px; font-weight: bold;",
                    "{breed_info}"
                }

                p { style: "color: #888; font-size: 14px; margin-top: 10px;", "{analysis_result}" }

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
                        let breed_result = process_image.read_async().await;
                        if let Ok(breed) = &*breed_result {
                            let mut analysis = analysis_result.write();
                            let old_analysis = analysis.clone();
                            *analysis = format!("Manual trigger: Analyzing {} breed...", breed);
                            drop(breed_result);
                            drop(analysis);
                            document::eval(
                                    r#"await new Promise(resolve => setTimeout(resolve, 2000)); return null;"#,
                                )
                                .await
                                .unwrap();
                            let mut analysis = analysis_result.write();
                            *analysis = old_analysis;
                        }
                    },
                    style: "background: #2196F3; color: white; border: none; padding: 12px 24px; font-size: 16px; border-radius: 4px; cursor: pointer;",
                    "🔍 Re-analyze Breed"
                }
            }

            div { style: "margin-top: 30px; padding: 15px; background: #e3f2fd; border-left: 4px solid #2196F3; text-align: left;",
                h3 { "📊 Resource Dependency Graph:" }
                pre { style: "font-family: monospace; line-height: 1.6; margin: 10px 0;",
                    "dog_image (base)\n"
                    "    ├─→ process_image (extracts breed)\n"
                    "    └─→ breed_analyzer (analyzes URL)"
                }
                p { style: "margin-top: 10px; font-size: 14px;",
                    "Both process_image and breed_analyzer depend on dog_image, "
                    "demonstrating how multiple resources can independently use "
                    code { "read_async()" }
                    " on the same base resource."
                }
            }
        }
    }
}
