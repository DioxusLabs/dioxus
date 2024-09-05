use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        document::Stylesheet { href: asset!("/assets/style.css") }
        document::Stylesheet { href: asset!("/assets/tailwind.css") }

        div { class: "w-full h-full text-center my-20",
            h1 { class: "text-4xl font-bold", "Dioxus iOS apps!" }
            h3 { class: "sparkles", "!!zero-xcode!!!!!!!!!" }
            button { onclick: move |_| println!("High five!"), "High five!" }
        }

        div { class: "w-full h-full flex flex-col mx-auto space-y-12",
            ImageList { dogs: 3, style: "fluffy" }
        }
    }
}

#[component]
fn ImageList(dogs: ReadOnlySignal<i32>, style: String) -> Element {
    rsx! {
        for i in 0..dogs() {
            ul { class: "flex flex-row justify-center",
                img {
                    src: "/assets/dogs/{style}/dog{i}.jpg",
                    height: "200px",
                    border_radius: "10px",
                }
            }
        }
    }
}
