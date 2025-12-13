//! Chaining Resources with `read_async()`
//!
//! This example demonstrates how to use `read_async()` to chain resources together while
//! maintaining guards across await points. The key benefit is that you can pass values
//! (like write guards) to `read_async()`, and it will return them back after the await,
//! reminding you that you're holding these guards across an await point and helping prevent
//! common async pitfalls.

use core::error;

use dioxus::{core::Runtime, prelude::*};

fn main() {
    dioxus::launch(app)
}

fn app() -> Element {
    let mut image_url = use_signal(|| "No Image".to_owned());
    let mut breed_info = use_signal(|| "No Breed Info".to_owned());
    let mut request_count = use_signal(|| 0);

    #[derive(serde::Deserialize, Clone)]
    struct DogImageResponse {
        /// URL of the dog image
        message: String,
    }

    let mut dog_image = use_resource(move || async move {
        reqwest::get("https://dog.ceo/api/breeds/image/random")
            .await?
            .json::<DogImageResponse>()
            .await
    });

    let _process_image = use_resource(move || async move {
        // Guards we want to hold across the await point
        let mut image_url_write = image_url.write();
        let mut breed_info_write = breed_info.write();

        *image_url_write = "Loading...".to_owned();
        *breed_info_write = "Waiting for dog image...".to_owned();
        drop(image_url_write);
        drop(breed_info_write);

        // ⚠️ WARNING: Normally, holding guards across await points is dangerous!
        // `read_async()` solves this by:
        // 1. Forcing you to acknowledge you have some or no guards and accepting them as parameters
        // 2. Dropping them if the resource is currently unavailable
        // 3. Returning them back with the resource if the resource is available

        // Wait for the first resource and get guards back
        let result_ref = dog_image.read_async().await;
        let mut image_url_write = image_url.write();
        let mut breed_info_write = breed_info.write();

        let mut count_write = request_count.write();
        // Now we can safely use the result and our write guards together
        match &*result_ref {
            Ok(response) => {
                let url = &response.message;
                *image_url_write = url.clone();

                if let Some(breed_part) = url.split("/breeds/").nth(1)
                    && let Some(breed) = breed_part.split('/').next()
                {
                    let formatted_breed = breed.replace('-', " ").to_uppercase();
                    *breed_info_write = format!("Breed: {}", formatted_breed);
                } else {
                    *breed_info_write = "Breed information not found".to_string();
                }

                *count_write += 1;
            }
            Err(e) => {
                *image_url_write = format!("Error loading image: {}", e);
                *breed_info_write = "Failed to fetch breed info".to_string();
            }
        }

        Ok::<(), anyhow::Error>(())
    });

    rsx! {
        button {
            onclick: move |_| async move {
                error!("Hit 1");
                let test = _process_image.read_async().await;
                error!("Hit 2");
            },
            "Click me"
        }
        div { style: "font-family: Arial, sans-serif; max-width: 600px; margin: 50px auto; text-align: center;",

            h1 { "🐕 Random Dog Image Fetcher" }

            p { style: "color: #666; margin-bottom: 20px;",
                "This example demonstrates chaining resources with "
                code { "read_async()" }
                " to safely handle guards across await points."
            }

            div { style: "background: #f5f5f5; padding: 20px; border-radius: 8px; margin: 20px 0;",

                // Display the fetched image
                match &*dog_image.value().read() {
                    Some(Ok(_)) => rsx! {
                        img {
                            src: "{image_url}",
                            alt: "Random dog",
                            style: "max-width: 100%; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1);",
                        }
                    },
                    Some(Err(_)) => rsx! {
                        p { style: "color: red;", "❌ {image_url}" }
                    },
                    None => rsx! {
                        p { "⏳ Loading dog image..." }
                    },
                }

                // Display breed information
                p { style: "margin-top: 15px; font-size: 18px; font-weight: bold;",
                    "{breed_info}"
                }

                p { style: "color: #888; font-size: 14px;", "Images fetched: {request_count}" }
            }

            button {
                onclick: move |_| dog_image.restart(),
                style: "background: #4CAF50; color: white; border: none; padding: 12px 24px; font-size: 16px; border-radius: 4px; cursor: pointer;",
                "🔄 Fetch Another Dog"
            }

            div { style: "margin-top: 30px; padding: 15px; background: #fff3cd; border-left: 4px solid #ffc107; text-align: left;",

                h3 { "💡 How read_async() works:" }
                ul { style: "text-align: left; line-height: 1.8;",
                    li {
                        strong { "Accepts values: " }
                        "Pass guards or other values as parameters"
                    }
                    li {
                        strong { "Waits safely: " }
                        "Drops guards while waiting, preventing double borrow"
                    }
                    li {
                        strong { "Returns values: " }
                        "Returns the resource result AND your values back together if the resource is ready"
                    }
                    li {
                        strong { "Compiler reminder: " }
                        "Forces acknowledgment that you're holding guards across await"
                    }
                }
            }
        }
    }
}
