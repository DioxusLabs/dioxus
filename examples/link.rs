use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! (
        div {
            p {
                a {
                    href: "http://dioxuslabs.com/",
                    "default link"
                }
            }
            p {
                a {
                    href: "http://dioxuslabs.com/",
                    "browser-open": "false",
                    onclick: |_| {
                        println!("Hello Dioxus");
                    },
                    "custom event link",
                }
            }
        }
    ))
}
