use dioxus::prelude::*;

fn main() {
    println!("[server] Launching app!");

    dioxus::launch(app);
}

fn app() -> Element {
    let mut favorite_dog = use_signal(|| None);

    rsx! {
        document::Stylesheet { href: asset!("/assets/style.css") }
        document::Stylesheet { href: asset!("/assets/tailwind.css") }

        div { class: "w-full h-full text-center my-20",
            h1 { class: "text-4xl font-bold", "Dioxus iOS apps!" }
            button {
                onclick: move |_| async move {
                    let dog = get_random_dog().await.unwrap_or_else(|err| format!("Error: {err}"));
                    favorite_dog.set(Some(dog));
                },
                "New favorite dog!"
            }
        }

        div { class: "w-full h-full flex flex-col mx-auto space-y-12",
            if let Some(favorite_dog) = favorite_dog() {
                img { src: "{favorite_dog}", max_width: "500px", height: "500px", margin: "auto" }
            }
            ImageList { dogs: 3, style: "cute" }
        }
    }
}

#[component]
fn ImageList(dogs: ReadOnlySignal<i32>, style: String) -> Element {
    rsx! {
        for i in 0..dogs() {
            ul { class: "flex flex-row justify-center",
                img {
                    src: "/assets/dogs/{style}/dog{i}.jpg",
                    height: "200px",
                    border_radius: "10px",
                }
            }
        }
    }
}

#[server(endpoint = "get_random_dog")]
async fn get_random_dog() -> Result<String, ServerFnError> {
    let breed = "husky".to_string();
    println!("Getting a random dog from the server!");

    #[derive(serde::Deserialize, Debug)]
    struct DogApi {
        message: String,
    }

    let dog = reqwest::get(format!("https://dog.ceo/api/breed/{breed}/images/random"))
        .await
        .unwrap()
        .json::<DogApi>()
        .await?;

    Ok(dog.message)
}
