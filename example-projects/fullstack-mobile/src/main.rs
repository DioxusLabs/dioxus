use dioxus::prelude::*;

fn main() {
    println!("[server] Launching app!");

    dioxus::launch(app);
}

fn app() -> Element {
    let mut favorite_dog = use_signal(|| "Fido".to_string());

    rsx! {
        document::Stylesheet { href: asset!("/assets/style.css") }
        document::Stylesheet { href: asset!("/assets/tailwind.css") }

        div { class: "w-full h-full text-center my-20",
            h1 { class: "text-4xl font-bold", "Dioxus iOS apps!" }
            h3 { class: "sparkles", "Favorite dog: {favorite_dog}" }
            button { onclick: move |_| async move {
                let dog = get_random_dog().await.unwrap_or_else(|err| format!("Error: {err}"));
                favorite_dog.set(dog);
            }, "New favorite dog!" }
        }

        div { class: "w-full h-full flex flex-col mx-auto space-y-12",
            ImageList { dogs: 3, style: "fluffy" }
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
    println!("Getting a random dog from the server!");
    tracing::info!("Getting a random dog from the server!");
    let breed = "husky".to_string();

    Ok(breed)

    // #[derive(serde::Deserialize, Debug)]
    // struct DogApi {
    //     message: String,
    // }

    // let dog = reqwest::get(format!("https://dog.ceo/api/breed/{breed}/images/random"))
    //     .await
    //     .unwrap()
    //     .json::<DogApi>()
    //     .await?;

    // Ok(dog.message)
}
