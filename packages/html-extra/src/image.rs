use dioxus::prelude::{manganis::ImageAsset, *};

/// The source of an image
#[derive(PartialEq, Clone, Debug)]
pub enum ImageSource<'a> {
    Url(&'a str),
    Asset(ImageAsset),
}

impl<'a> From<&'a str> for ImageSource<'a> {
    fn from(url: &'a str) -> Self {
        ImageSource::Url(url)
    }
}

impl From<ImageAsset> for ImageSource<'_> {
    fn from(asset: ImageAsset) -> Self {
        ImageSource::Asset(asset)
    }
}

/// The props for the [`Image`] component
#[derive(Props)]
pub struct ImageProps<'a> {
    #[props(into)]
    src: ImageSource<'a>,
    alt: &'a str,
    #[props(extends = GlobalAttributes)]
    extra_attributes: Vec<Attribute<'a>>,
}

/// A component that renders an image. If the image is an asset optimized by manganis, this component will render a low quality preview first, then replace it with the full quality image when it is loaded.
pub fn Image<'a>(cx: Scope<'a, ImageProps<'a>>) -> Element<'a> {
    let main_src = match &cx.props.src {
        ImageSource::Url(url) => url,
        ImageSource::Asset(asset) => asset.path(),
    };

    let style = match &cx.props.src {
        ImageSource::Url(_) => Default::default(),
        ImageSource::Asset(asset) => match asset.preview() {
            Some(preview) => format!("background-repeat: no-repeat; background-size: cover; background-image: url('{}');", preview),
            None => Default::default()
        }
    };

    render! {
        img {
            src: "{main_src}",
            style: "{style}",
            alt: "{cx.props.alt}",
            ..cx.props.extra_attributes,
        }
    }
}
