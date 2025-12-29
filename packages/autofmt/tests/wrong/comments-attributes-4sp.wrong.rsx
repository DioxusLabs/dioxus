//! hello

#[component]
fn App() -> Element {
    rsx! {
        Stylesheet { href: MAIN_CSS }
        div { id: "hero",
            // ss
            // s
            // asd
            img {
                // blah
                src: HEADER_SVG, id: "header" }
            div { id: "links",
                a { href: "https://discord.gg/XgGxMSkvUM", "👋 Community Discord" }
                div {
                    // blah



                    src: HEADER_SVG, id: "header"
                }
            }
        }
    }
}
