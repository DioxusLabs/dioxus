use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let contents = use_state(&cx, || String::from("<script>alert(123)</script>"));

    cx.render(rsx! {
        div {
            "hello world!"

            h1 { "{contents}" }

            h3 { [contents.as_str()] }

            input {
                value: "{contents}",
                oninput: move |e| {
                    contents.set(e.value.clone());
                    eprintln!("asd");
                },
                "type": "text",
            }
        }
    })
}
