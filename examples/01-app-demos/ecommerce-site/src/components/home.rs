// The homepage is statically rendered, so we don't need to a persistent websocket connection.

use crate::{
    api::{fetch_products, Sort},
    components::nav::Nav,
    components::product_item::ProductItem,
};
use dioxus::prelude::*;

pub(crate) fn Home() -> Element {
    let products = use_loader(|| fetch_products(10, Sort::Ascending))?;

    rsx! {
        Nav {}
        section { class: "p-10",
            for product in products.iter() {
                ProductItem {
                    product: product.clone()
                }
            }
        }
    }
}
