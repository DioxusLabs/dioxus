//! This example shows to wrap a webcomponent / custom element with a component.
//!
//! Oftentimes, a third party library will provide a webcomponent that you want
//! to use in your application. This example shows how to create that custom element
//! directly with the raw_element method on NodeFactory.

use dioxus::prelude::*;

fn main() {
    let mut dom = VirtualDom::new(app);
    let _ = dom.rebuild();

    let output = dioxus_ssr::render_vdom(&dom);

    println!("{}", output);
}

custom_elements! {
    my_element("my-element", name);
    your_element("your-element", foo, bar);
    other_element("other-element",);
}

fn app(cx: Scope) -> Element {
    render! {
        div { "built-in element" },
        my_element { name: "bob", title: "global attribute works", "custom element" }
        your_element { foo: "foo", bar: "bar" }
        other_element { "other element" }
    }
}
