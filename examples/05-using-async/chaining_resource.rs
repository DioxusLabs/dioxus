//! Suspense in Dioxus
//!
//! Suspense allows components to bubble up loading states to parent components, simplifying data fetching.

use anyhow::anyhow;
use dioxus::prelude::*;

fn main() {
    dioxus::launch(app)
}

fn app() -> Element {
    let mut signal1 = use_signal(|| "empty".to_string());
    let mut signal2 = use_signal(|| "empty".to_string());

    #[derive(serde::Deserialize, serde::Serialize, PartialEq)]
    struct DogApi {
        message: String,
    }
    let message: Resource<anyhow::Result<DogApi>> = use_resource(move || async move {
        let dog_api = reqwest::get("https://dog.ceo/api/breeds/image/random/")
            .await?
            .json::<DogApi>()
            .await?;
        Ok(dog_api)
    });

    let value: Resource<anyhow::Result<()>> = use_resource(move || async move {
        let signal1_write = signal1.write();
        let signal2_write = signal2.write();
        let (message, mut signal1_write, mut signal2_write) =
            message.read_async((signal1_write, signal2_write)).await;
        let dog_api = message.as_ref().map_err(|e| anyhow!("{}", e))?;
        *signal1_write = dog_api.message.clone();
        *signal2_write = dog_api.message.clone();
        Ok(())
    });

    rsx! {}
}
