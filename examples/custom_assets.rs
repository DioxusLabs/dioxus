use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            p {
                "This should show an image:"
            }
            img { src: manganis::mg!(image("examples/assets/logo.png").format(ImageType::Avif)).to_string() }
        }
    }
}
