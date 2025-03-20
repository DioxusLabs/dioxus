//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

use dioxus::prelude::*;

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(server_only!(
            ServeConfig::builder().enable_out_of_order_streaming()
        ))
        .launch(app);
}

fn app() -> Element {
    rsx! { Router::<Route> {} }
}

#[derive(Clone, Routable, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
enum Route {
    #[route("/")]
    Home {},

    #[route("/:breed")]
    Breed { breed: String },
}

#[component]
fn Home() -> Element {
    rsx! {
        Link { to: Route::Breed { breed: "hound".to_string() }, "Hound" }
    }
}

#[component]
fn Breed(breed: String) -> Element {
    rsx! {
        BreedGallery { breed: "{breed}", slow: false }
        SuspenseBoundary {
            fallback: |_| rsx! { "Loading..." },
            DoesNotSuspend {}
            BreedGallery { breed, slow: true }
        }
    }
}

#[component]
fn DoesNotSuspend() -> Element {
    rsx! { "404" }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct BreedResponse {
    message: Vec<String>,
}

#[component]
fn BreedGallery(breed: ReadOnlySignal<String>, slow: bool) -> Element {
    // use_server_future is very similar to use_resource, but the value returned from the future
    // must implement Serialize and Deserialize and it is automatically suspended
    let response = use_server_future(move || async move {
        if slow {
            #[cfg(feature = "server")]
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
        #[cfg(feature = "server")]
        {
            use http::StatusCode;
            let context = server_context();
            let mut write = context.response_parts_mut();
            write.status = StatusCode::NOT_FOUND;
            write.extensions.insert("error???");
            write.version = http::Version::HTTP_2;
            write
                .headers
                .insert("x-custom-header", http::HeaderValue::from_static("hello"));
        }
        // The future will run on the server during SSR and then get sent to the client
        reqwest::Client::new()
            .get(format!("https://dog.ceo/api/breed/{breed}/images"))
            .send()
            .await
            // reqwest::Result does not implement Serialize, so we need to map it to a string which
            // can be serialized
            .map_err(|err| err.to_string())?
            .json::<BreedResponse>()
            .await
            .map_err(|err| err.to_string())
        // use_server_future calls `suspend` internally, so you don't need to call it manually, but you
        // do need to bubble up the suspense variant with `?`
    })?;

    // If the future was still pending, it would have returned suspended with the `?` above
    // we can unwrap the None case here to get the inner result
    let response_read = response.read();
    let response = response_read.as_ref().unwrap();

    // Then you can just handle the happy path with the resolved future
    rsx! {
        div {
            display: "flex",
            flex_direction: "row",
            match response {
                Ok(urls) => rsx! {
                    for image in urls.message.iter().take(3) {
                        img {
                            src: "{image}",
                            width: "100px",
                            height: "100px",
                        }
                    }
                },
                Err(err) => rsx! { "Failed to fetch response: {err}" },
            }
        }
    }
}
