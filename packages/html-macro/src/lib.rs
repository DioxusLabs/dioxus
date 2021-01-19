extern crate proc_macro;

use crate::parser::HtmlParser;
use crate::tag::Tag;
use syn::parse::{Parse, ParseStream, Result};
use syn::parse_macro_input;

mod parser;
mod tag;
pub(crate) mod validation;

/// Used to generate VirtualNode's from a TokenStream.
///
/// html! { <div> Welcome to the html! procedural macro! </div> }
#[proc_macro]
pub fn html(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed = parse_macro_input!(input as Html);

    let mut html_parser = HtmlParser::new();

    let parsed_tags_len = parsed.tags.len();

    // Iterate over all of our parsed tags and push them into our HtmlParser one by one.
    //
    // As we go out HtmlParser will maintain some heuristics about what we've done so far
    // since that will sometimes inform how to parse the next token.
    for (idx, tag) in parsed.tags.iter().enumerate() {
        let mut next_tag = None;

        if parsed_tags_len - 1 > idx {
            next_tag = Some(&parsed.tags[idx + 1])
        }

        html_parser.push_tag(tag, next_tag);
    }

    html_parser.finish().into()
}

#[derive(Debug)]
struct Html {
    tags: Vec<Tag>,
}

impl Parse for Html {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut tags = Vec::new();

        while !input.is_empty() {
            let tag: Tag = input.parse()?;
            tags.push(tag);
        }

        Ok(Html { tags })
    }
}
