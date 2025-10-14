//! This example showcases a fullstack variant of the "dog app" demo, but with the loader and actions
//! self-hosted instead of using the Dog API.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // Fetch the list of breeds from the Dog API, using the `?` syntax to suspend or throw errors
    let breed_list = use_loader(list_breeds)?;

    // Whenever this action is called, it will re-run the future and return the result.
    let mut breed = use_action(get_random_breed_image);

    rsx! {
        h1 { "Doggo selector" }
        div { width: "400px",
            for cur_breed in breed_list.read().iter().take(20).cloned() {
                button { onclick: move |_| { breed.call(cur_breed.clone()); }, "{cur_breed}" }
            }
        }
        div {
            match breed.value() {
                None => rsx! { div { "Click the button to fetch a dog!" } },
                Some(Err(_e)) => rsx! { div { "Failed to fetch a dog, please try again." } },
                Some(Ok(res)) => rsx! { img { max_width: "500px", max_height: "500px", src: "{res}" } },
            }
        }

    }
}

#[get("/api/breeds/list/all")]
async fn list_breeds() -> Result<Vec<String>> {
    Ok(vec!["bulldog".into(), "labrador".into(), "poodle".into()])
}

#[get("/api/breed/{breed}/images/random")]
async fn get_random_breed_image(breed: String) -> Result<String> {
    match breed.as_str() {
        "bulldog" => Ok("https://images.dog.ceo/breeds/buhund-norwegian/hakon3.jpg".into()),
        "labrador" => Ok("https://images.dog.ceo/breeds/labrador/n02099712_2501.jpg".into()),
        "poodle" => Ok("https://images.dog.ceo/breeds/poodle-standard/n02113799_5973.jpg".into()),
        _ => HttpError::not_found("Breed not found")?,
    }
}
