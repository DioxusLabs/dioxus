use dioxus::prelude::*;

/// Make sure that rsx! handles conditional attributes with one formatted branch correctly
/// Regression test for https://github.com/DioxusLabs/dioxus/issues/2997
#[test]
fn partially_formatted_conditional_attribute() {
    let width = "1px";
    _ = rsx! {
        div {
            width: if true { "{width}" } else { "100px" }
        }
    };
}
