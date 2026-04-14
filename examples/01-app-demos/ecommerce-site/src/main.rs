#![allow(non_snake_case)]

use components::home::Home;
use components::loading::ChildrenOrLoading;
use dioxus::prelude::*;

mod components {
    pub mod error;
    pub mod home;
    pub mod loading;
    pub mod nav;
    pub mod product_item;
    pub mod product_page;
}
mod api;

fn main() {
    dioxus::launch(|| {
        rsx! {
            document::Link {
                rel: "stylesheet",
                href: asset!("/public/tailwind.css")
            }

            ChildrenOrLoading {
                Router::<Route> {}
            }
        }
    });
}

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},

    #[route("/details/:product_id")]
    Details { product_id: usize },
}

#[component]
/// Render a more sophisticated page with ssr
fn Details(product_id: usize) -> Element {
    rsx! {
        div {
            components::nav::Nav {}
            components::product_page::ProductPage {
                product_id
            }
        }
    }
}
