use dioxus::component::Component;
use dioxus::events::on::MouseEvent;
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

fn main() {}

fn html_usage() {
    let r = rsx! {
        div {
            onclick: move |_| {}
            "hello world"
        }
    };
}
