use dioxus::prelude::*;

#[test]
fn basic_syntax_is_a_template() {
    //
    let var = 123;
    let asd = 123;

    let g = rsx! {
        div {
            class: "asd",
            class: "{asd}",
            onclick: move |_| {},
            div { "{var}" }
        }
    };
}
