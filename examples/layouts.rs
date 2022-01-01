use std::collections::HashMap;

use dioxus::{core::ElementId, prelude::*};
use rink::TuiNode;

fn main() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild();

    let mut layout = stretch2::Stretch::new();
    let mut nodes = HashMap::new();
    rink::collect_layout(&mut layout, &mut nodes, &dom, dom.base_scope().root_node());

    let node = nodes
        .remove(&dom.base_scope().root_node().mounted_id())
        .unwrap();

    layout
        .compute_layout(node.layout, stretch2::geometry::Size::undefined())
        .unwrap();

    for (id, node) in nodes.drain() {
        println!("{:?}", layout.layout(node.layout));
    }
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            width: "100%",
            height: "100%",
            flex_direction: "column",

            div {
                "hi"
            }
            div {
                "bi"
                "bi"
            }
        }
    })
}

// fn print_layout(mut nodes: HashMap<ElementId, TuiNode>, node: &VNode) {
//     match node {
//         VNode::Text(_) => todo!(),
//         VNode::Element(_) => todo!(),
//         VNode::Fragment(_) => todo!(),
//         VNode::Component(_) => todo!(),
//         VNode::Placeholder(_) => todo!(),
//     }
// }
