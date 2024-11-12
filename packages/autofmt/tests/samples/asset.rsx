rsx! {
    div { "hi" }
    img {
        // note: the line breaking here is weird but it's because of prettyplease, not dioxus-autofmt
        // we might want to fix this in the future
        src: asset!(
            "/assets/logo.png".image().size(512, 512).format(ImageType::Jpg).url_encoded()
            .image().size(512, 512).format(ImageType::Jpg).url_encoded()
        ),
        alt: "logo",
    }
}
