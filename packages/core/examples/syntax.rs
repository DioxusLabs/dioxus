use dioxus::component::Component;
use dioxus::events::on::MouseEvent;
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

fn main() {}

fn html_usage() {
    let mo = move |_| {};
    let r = rsx! {
        div {
            onclick: move |_| {}
            onmouseover: {mo}
            "type": "bar",
            "hello world"
        }
    };

    let items = ["bob", "bill", "jack"];

    let f = items.iter().filter(|f| f.starts_with("b")).map(|f| {
        rsx! {
            "hello {f}"
        }
    });

    let p = rsx! {
        div {
            {f}
        }
    };
}
