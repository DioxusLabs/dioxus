//! some basic test cases with nested rsx!

fn App() -> Element {
    let mut count = use_signal(|| 0);
    let mut text = use_signal(|| "...".to_string());

    rsx! {
        Component {
            header: rsx! {
                h1 { "hi" }
                h1 { "hi" }
            },
            onrender: move |_| {
                count += 1;
                rsx! {
                    div {
                        h1 { "hi" }
                        "something nested?"
                    }
                    div {
                        h2 { "hi" }
                        "something nested?"
                    }
                    div {
                        h3 { "hi" }
                        "something nested?"
                    }
                }
            }
        }
    }
}
