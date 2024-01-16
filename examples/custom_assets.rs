use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    render! {
        div {
            p {
                "This should show an image:"
            }
            img { src: manganis::mg!(image("examples/assets/logo.png").format(ImageType::Avif)).to_string() }
        }
    }
}
