// The homepage is statically rendered, so we don't need to a persistent websocket connection.

use crate::{
    api::{fetch_products, Sort},
    components::nav,
    components::product_item::product_item,
};
use dioxus::prelude::*;

pub(crate) fn Home() -> Element {
    let products = use_server_future(|| fetch_products(10, Sort::Ascending))?;
    let products = products().unwrap()?;

    rsx! {
        nav::nav {}
        section { class: "p-10",
            for product in products {
                product_item {
                    product
                }
            }
        }
    }
}
