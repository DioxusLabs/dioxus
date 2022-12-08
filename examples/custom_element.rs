//! This example shows to wrap a webcomponent / custom element with a component.
//!
//! Oftentimes, a third party library will provide a webcomponent that you want
//! to use in your application. This example shows how to create that custom element
//! directly with the raw_element method on NodeFactory.

use dioxus::prelude::*;

fn main() {
    let mut dom = VirtualDom::new(app);
    let _ = dom.rebuild();

    let output = dioxus_ssr::render(&dom);

    println!("{}", output);
}

fn app(cx: Scope) -> Element {
    // let nf = NodeFactory::new(cx);

    // let mut attrs = dioxus::core::exports::bumpalo::collections::Vec::new_in(nf.bump());

    // attrs.push(nf.attr("client-id", format_args!("abc123"), None, false));

    // attrs.push(nf.attr("name", format_args!("bob"), None, false));

    // attrs.push(nf.attr("age", format_args!("47"), None, false));

    // Some(nf.raw_element("my-element", None, &[], attrs.into_bump_slice(), &[], None))

    todo!()
}
