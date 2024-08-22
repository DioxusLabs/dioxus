//! This example shows how to add metadata to the page with the Meta component

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! {
        // You can use the Meta component to render a meta tag into the head of the page
        // Meta tags are useful to provide information about the page to search engines and social media sites
        // This example sets up meta tags for the open graph protocol for social media previews
        document::Meta {
            property: "og:title",
            content: "My Site",
        }
        document::Meta {
            property: "og:type",
            content: "website",
        }
        document::Meta {
            property: "og:url",
            content: "https://www.example.com",
        }
        document::Meta {
            property: "og:image",
            content: "https://example.com/image.jpg",
        }
        document::Meta {
            name: "description",
            content: "My Site is a site",
        }
    }
}
