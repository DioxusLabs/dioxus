use dioxus_ssr::prelude::{Context, VNode};

// Parse a snippet into
pub fn markdown_to_snippet<'a, P>(cx: Context<'a, P>, text: &str) -> Vec<VNode<'a>> {
    let snips = Vec::new();
    use pulldown_cmark::{Options, Parser};
    let mut options = Options::empty();
    let mut parser = Parser::new_ext(text, options);

    while let Some(evt) = parser.next() {
        match evt {
            pulldown_cmark::Event::Start(tag) => {
                // take until the end
                let r = parser.next();
            }

            // push a p{} tag with the contents
            pulldown_cmark::Event::Text(text) => todo!(),

            // Code delinates an end
            pulldown_cmark::Event::Code(code) => todo!(),

            // not supported
            pulldown_cmark::Event::Html(ht) => {}
            _ => {}
        }
    }

    //
    snips
}
