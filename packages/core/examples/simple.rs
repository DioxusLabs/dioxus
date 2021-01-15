use std::future::Future;

use dioxus_core::{component::AnyContext, prelude::*};
use virtual_dom_rs::Closure;

pub fn main() {
    let dom = VirtualDom::new(root);
    let mut renderer = TextRenderer::new(dom);
    let output = renderer.render();
}

fn root(ctx: &mut AnyContext) -> VirtualNode {
    // the regular html syntax

    // html! {
    //     <html>
    //         <Head />
    //         <Body />
    //         <Footer />
    //     </html>
    // }

    // or a manually crated vnode
    {
        let mut node_0 = VirtualNode::element("div");
        let mut node_1: IterableNodes = ("Hello world!").into();
        node_1.first().insert_space_before_text();
        let mut node_2 = VirtualNode::element("button");
        {
            // let closure = Closure::wrap(Box::new(|_| {}) as Box<FnMut(_)>);
            // let closure_rc = std::rc::Rc::new(closure);
            // node_2
            //     .as_velement_mut()
            //     .expect("Not an element")
            //     .events
            //     .0
            //     .insert("onclick".to_string(), closure_rc);
        }

        if let Some(ref mut element_node) = node_0.as_velement_mut() {
            element_node.children.extend(node_1.into_iter());
        }
        if let Some(ref mut element_node) = node_0.as_velement_mut() {
            element_node.children.extend(node_2.into_iter());
        }

        VirtualNode

        node_0
    }
}

fn Head(ctx: &mut AnyContext) -> VirtualNode {
    html! {
        <head>
            {"Head Section"}
        </head>
    }
}

fn Body(ctx: &mut AnyContext) -> VirtualNode {
    html! {
        <body>
            {"Footer Section"}
        </body>
    }
}

fn Footer(ctx: &mut AnyContext) -> VirtualNode {
    html! {
        <div>
            {"Footer Section"}
        </div>
    }
}
