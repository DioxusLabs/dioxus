use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;

#[test]
fn diffing_works() {}

#[test]
fn html_and_rsx_generate_the_same_output() {
    let old = rsx! {
        div { "Hello world!" }
    };

    // let new = html! {
    //     <div>"Hello world!"</div>
    // };
}
