// The homepage is statically rendered, so we don't need to a persistent websocket connection.

use crate::{
    api::{fetch_products, Sort},
    block_on,
    components::nav,
    components::product_item::product_item,
};
use dioxus::prelude::*;

pub(crate) fn Home(cx: Scope) -> Element {
    let products = cx.use_hook(|| block_on(fetch_products(10, Sort::Ascending)));

    cx.render(rsx!(
        head {
            link {
                rel: "stylesheet",
                href: "/public/tailwind.css"
            }
        }
        body {
            nav::nav {}
            section { class: "p-10",
                products.iter().flatten().map(|product| rsx!{
                    product_item {
                        product: product.clone()
                    }
                })
            }
        }
    ))
}
