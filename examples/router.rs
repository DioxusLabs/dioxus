//! Dioxus Router
//! -------------
//!
//! This exmaple showcases the Dioxus Router hook. This hook makes it possible to using the browser's navigation API to
//! display different content based on the Page's URL. The hook provides a configuration object that calls various
//! specified callbacks whenever the page URL changes. Using this hook should feel like building a "URL directory", similar
//! to how Tide handles paths.

use dioxus::prelude::*;

fn main() {
    diouxs_webview::launch(App).run().await
}

fn App(ctx: Context<()>) -> VNode {
    let router = use_router(&ctx, |router| {
        //
        router.get("/dogs/:dogId/").render(|ctx, request| {
            rsx! {
                div {

                }
            }
        });

        router.get("/cats/:catId/").render(|ctx, request| {
            rsx! {
                div {

                }
            }
        });
    });

    ctx.render(rsx! {
        div {
            a { href="/dogs/"}
            a { href="/cats/"}
            {router.render()}
        }
    })
}
