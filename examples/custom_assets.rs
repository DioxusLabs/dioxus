use dioxus::prelude::*;

#[cfg(not(feature = "collect-assets"))]
static ASSET_PATH: &str = "examples/assets/logo.png";

#[cfg(feature = "collect-assets")]
static ASSET_PATH: &str =
    manganis::mg!(image("examples/assets/logo.png").format(ImageType::Avif)).path();

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            p { "This should show an image:" }
            img { src: ASSET_PATH.to_string() }
        }
    }
}
