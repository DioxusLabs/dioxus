//! The docs generator takes in the `docs` folder and creates a neat, statically-renderer webpage.
//! These docs are used to generate the public-facing doc content, showcasing Dioxus' abiltity to
//! be used in custom static rendering pipelines.
//!
//! We use pulldown_cmark as the markdown parser, but instead of outputting html directly, we output
//! VNodes to be used in conjuction with our custom templates.

use dioxus::core::prelude::*;
use pulldown_cmark::{Options, Parser};

fn main() {
    let gen_dir = "../docs/";

    let site: FC<()> = |_| {
        html! {
            <html>

            <head>
            </head>
            <body>
            </body>

            </html>
        }
    };
}

static Homepage: FC<()> = |_| {
    html! {<div> </div>}
};

static DocPage: FC<()> = |_| {
    html! {<div> </div>}
};

// struct StaticSiteCfg {
//     gen_dir: &'static str,
//     homepage_template: fn() -> VNode,
//     page_template: fn(page: &'static str) -> VNode,
// }

// impl StaticSiteCfg {
//     fn render(self) -> anyhow::Result<VNode> {
//         let StaticSiteCfg { .. } = self;

//         // Set up options and parser. Strikethroughs are not part of the CommonMark standard
//         // and we therefore must enable it explicitly.
//         let mut options = Options::empty();
//         options.insert(Options::ENABLE_STRIKETHROUGH);
//         let parser = Parser::new_ext(markdown_input, options);

//         //

//         Ok(html! {<div> </div>})
//     }
// }
