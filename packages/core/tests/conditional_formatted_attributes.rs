use dioxus::prelude::*;

/// Make sure that rsx! handles conditional attributes with one formatted branch correctly
/// Regression test for https://github.com/DioxusLabs/dioxus/issues/2997. There are no assertions in
/// this test as it primarily checks that the RSX can be compiled correctly.
#[test]
fn partially_formatted_conditional_attribute() {
    let width = "1px";
    _ = rsx! {
        div {
            width: if true { "{width}" } else { "100px" }
        }
    };

    // And make sure it works if one of those branches is an expression
    // Regression test for https://github.com/DioxusLabs/dioxus/issues/3146
    let opt = "button";

    _ = rsx! {
        input {
            type: if true { opt } else { "text" },
        }
        input {
            type: if true { opt.to_string() } else { "text with" },
        }
        input {
            type: if true { opt.to_string() } else { "text with {width}" },
        }
        input {
            type: if true { opt.to_string() } else if true { "" } else { "text with {width}" },
        }
        input {
            type: if true { "one" } else if true { "two" } else { "three" },
        }
        input {
            type: if true { "one" } else if true { "two" } else if true { "three" } else { opt },
        }
        input {
            type: if true { "one" } else if true { if false { "true" } else { "false" } } else { "three" },
        }
        input {
            type: if true { "one".to_string() },
        }
        input {
            type: if true { "one" },
        }
    };
}
