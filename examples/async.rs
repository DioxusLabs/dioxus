//! Example: README.md showcase
//!
//! The example from the README.md

use std::pin::Pin;

use dioxus::prelude::*;
use futures::Future;
fn main() {
    dioxus::web::launch(App)
}

#[derive(serde::Deserialize)]
struct DogApi {
    message: String,
}

const ENDPOINT: &str = "https://dog.ceo/api/breeds/image/random";

static App: FC<()> = |cx| {
    // let mut count = use_state(cx, || 0);
    let mut fut = cx.use_hook(
        move || {
            Box::pin(async {
                //
                loop {
                    // repeatadly get new doggos
                    match surf::get(ENDPOINT).recv_json::<DogApi>().await {
                        Ok(_) => (),
                        Err(_) => (),
                        // Ok(res) => rsx!(in cx, img { src: "{res.message}" }),
                        // Err(_) => rsx!(in cx, div { "No doggos for you :(" }),
                    }
                    // wait one seconds
                }
            }) as Pin<Box<dyn Future<Output = ()> + 'static>>
        },
        |h| h,
        |_| {},
    );

    cx.submit_task(fut);

    todo!()
    // cx.render(rsx! {
    //     div {
    //         h1 { "Hifive counter: {count}" }
    //         button { onclick: move |_| count += 1, "Up high!" }
    //         button { onclick: move |_| count -= 1, "Down low!" }
    //     }
    // })
};

// #[derive(serde::Deserialize)]
// struct DogApi {
//     message: String,
// }
// const ENDPOINT: &str = "https://dog.ceo/api/breeds/image/random";

// pub static App: FC<()> = |cx| {
//     let doggo = use_future_effect(&cx, move || async move {
//         match surf::get(ENDPOINT).recv_json::<DogApi>().await {
//             Ok(res) => rsx!(in cx, img { src: "{res.message}" }),
//             Err(_) => rsx!(in cx, div { "No doggos for you :(" }),
//         }
//     });

//     cx.render(rsx!(
//         div {
//             h1 {"Waiting for a doggo..."}
//             {doggo}
//         }
//     ))
// };
