//! This example demonstrates how to use the spread operator to pass attributes to child components.
//!
//! This lets components like the `Link` allow the user to extend the attributes of the underlying `a` tag.
//! These attributes are bundled into a `Vec<Attribute>` which can be spread into the child component using the `..` operator.

use dioxus::prelude::*;

fn main() {
    let dom = VirtualDom::prebuilt(app);
    let html = dioxus_ssr::render(&dom);

    println!("{}", html);
}

fn app() -> Element {
    rsx! {
        spreadable_component {
            width: "10px",
            extra_data: "hello{1}",
            extra_data2: "hello{2}",
            height: "10px",
            left: 1
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct Props {
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,

    extra_data: String,

    extra_data2: String,
}

fn spreadable_component(props: Props) -> Element {
    rsx! {
        audio { ..props.attributes, "1: {props.extra_data}\n2: {props.extra_data2}" }
    }
}
