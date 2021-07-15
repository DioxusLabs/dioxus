//! Example: README.md showcase
//!
//! The example from the README.md

use std::{pin::Pin, time::Duration};

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

static App: FC<()> = |cx| {
    let mut count = use_state(cx, || 0);
    let mut fut = Box::pin(async {
        let mut tick: i32 = 0;
        loop {
            async_std::task::sleep(Duration::from_millis(250)).await;
            log::debug!("ticking forward... {}", tick);
            tick += 1;
            // match surf::get(ENDPOINT).recv_json::<DogApi>().await {
            //     Ok(_) => (),
            //     Err(_) => (),
            // }
        }
    });

    cx.submit_task(fut);

    cx.render(rsx! {
        div {
            h1 {"it's working somewhat properly"}
        }
    })
};
