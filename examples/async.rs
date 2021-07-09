//! Example: README.md showcase
//!
//! The example from the README.md

use std::pin::Pin;

use dioxus::prelude::*;
use futures::Future;
fn main() {
    env_logger::init();
    log::info!("hello world");
    dioxus::desktop::launch(App, |c| c).expect("faield to launch");
}

#[derive(serde::Deserialize)]
struct DogApi {
    message: String,
}

const ENDPOINT: &str = "https://dog.ceo/api/breeds/image/random";

struct Ex(Pin<Box<dyn Future<Output = ()> + 'static>>);
static App: FC<()> = |cx| {
    // let mut count = use_state(cx, || 0);
    let mut fut = cx.use_hook(
        move || {
            Ex(Box::pin(async {
                //
                loop {
                    match surf::get(ENDPOINT).recv_json::<DogApi>().await {
                        Ok(_) => (),
                        Err(_) => (),
                    }
                }
            })
                as Pin<Box<dyn Future<Output = ()> + 'static>>)
        },
        |h| &mut h.0,
        |_| {},
    );

    cx.submit_task(fut);

    cx.render(rsx! {
        div {

        }
    })
};
