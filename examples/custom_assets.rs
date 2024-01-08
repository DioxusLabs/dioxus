use dioxus::prelude::{manganis::ImageAsset, *};

fn main() {
    dioxus_desktop::launch(app);
}

const LOGO: ImageAsset = mg!(image("examples/assets/logo.png").format(ImageType::Avif));

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            p { "This should show an image:" }
            img { src: "{LOGO}" }
        }
    })
}
