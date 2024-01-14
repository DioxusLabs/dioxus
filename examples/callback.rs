use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    let login = use_callback!(move |_| async move {
        let res = reqwest::get("https://dog.ceo/api/breeds/list/all")
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        println!("{res:#?}, ");
    });

    cx.render(rsx! {
        button { onclick: login, "Click me!" }
    })
}
