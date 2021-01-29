use std::future::Future;

use dioxus_core::{prelude::*, virtual_dom::Properties};
// use virtual_dom_rs::Closure;

// Stop-gap while developing
// Need to update the macro
type VirtualNode = VNode;

pub fn main() {
    let dom = VirtualDom::new_with_props(root);
    // let mut renderer = TextRenderer::new(dom);
    // let output = renderer.render();
}

#[derive(PartialEq)]
struct Props {
    name: String,
}
impl Properties for Props {}

fn root(ctx: &mut Context<Props>) -> VNode {
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
        let mut node_0 = VNode::element("div");
        {
            if let Some(ref mut element_node) = node_0.as_velement_mut() {
                // element_node.attrs.insert("blah", "blah");
                // element_node.children.extend(node_0.into_iter());
            }
        }

        let mut node_1: IterableNodes = ("Hello world!").into();

        node_1.first().insert_space_before_text();
        let mut node_2 = VNode::element("button");

        let node_3 = VNode::Component(VComponent::from_fn(
            Head,
            Props {
                name: "".to_string(),
            },
        ));

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

        node_0
    }
}

fn Head(ctx: &mut Context<Props>) -> VNode {
    html! {
        <head> "Head Section" </head>
    }
}

fn Body(ctx: &mut Context<Props>) -> VNode {
    html! {
        <body> {"Footer Section"}</body>
    }
}

fn Footer(ctx: &mut Context<Props>) -> VNode {
    let mut v = 10_i32;
    format!("Is this the real life, or is this fantasy, caught in a landslide");

    html! {
        <div>
            "Footer Section"
            "Footer Section"
            "Footer Section"
            "Footer Section"
            "Footer Section"
            "Footer Section"
        </div>
    }
}
