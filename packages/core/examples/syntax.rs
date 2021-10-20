use dioxus::component::Component;
use dioxus::events::on::MouseEvent;
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

fn main() {}

fn html_usage() {
    let r = html! {
        <div>
            "hello world"
            <div>
            </div>
            <div />
            "hello world"
            <Fragment>
            </Fragment>
        </div>
    };
    let r = rsx! {
        div {
            "hello world"
        }
    };
}

fn rsx_uage() {
    // let r = html! {
    //     <Fragment>
    //         "hello world"
    //         "hello world"
    //         "hello world"
    //         "hello world"
    //     </Fragment>
    // };
}
