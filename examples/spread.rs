use crate::dioxus_elements::ExtendedDivMarker;
use dioxus::{
    core::{exports::bumpalo::Bump, Attribute, HasAttributesBox},
    html::{ExtendedGlobalAttributesMarker, GlobalAttributesExtension},
    prelude::*,
};

fn main() {
    let mut dom = VirtualDom::new(app);
    let _ = dom.rebuild();
    let html = dioxus_ssr::render(&dom);

    println!("{}", html);
}

fn app(cx: Scope) -> Element {
    render! {
        Component {
            width: "10px",
            height: "10px",
            left: 1,
        }
    }
}

fn Component<'a>(cx: Scope<'a, Props<'a>>) -> Element<'a> {
    let attributes = &*cx.props.attributes;
    render! {
        audio {
            ..attributes,
        }
    }
}

#[derive(Props)]
struct Props<'a> {
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute<'a>>,
}
