use dioxus_core_macro::rsx;

pub fn main() {
    // render(rsx! {
    //     _ {}
    // });

    let g = String::from("asd");

    let lazy = rsx! {
        div {
            a: "asd",
            a: "asd",
            a: "asd",
            a: "asd",
            a: "asd",
            a: {rsx!{ h1 {"hello world"} }}, // include
            a: {g},
            b: {1 + 2},
            onclick: {move |_| {
                println!("hello world!")
            }},
            div {
                a: "asd"
                div {
                    div {
                        div {

                        }
                    }
                }
            }
            h1 {

            }
            h2 {
                "child"
            }
            "Childnode"
        }
    };

    render(lazy);
}

fn render(f: impl Fn(())) {}
