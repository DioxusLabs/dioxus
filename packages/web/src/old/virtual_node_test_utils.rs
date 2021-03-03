//! A collection of functions that are useful for unit testing your html! views.

use crate::VirtualNode;

impl VirtualNode {
    /// Get a vector of all of the VirtualNode children / grandchildren / etc of
    /// your virtual_node that have a label that matches your filter.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # #[macro_use] extern crate virtual_dom_rs;  fn main() {
    ///
    /// let component = html! {<div>
    ///  <span label="hello",> {"Hi!"} </span>
    ///  <em label="world",> {"There!!"} </em>
    ///  <em label="hello",></em>
    /// </div> };
    ///
    /// let hello_nodes = component.filter_label(|label| {
    ///     label.contains("hello")
    /// });
    ///
    /// assert_eq!(hello_nodes.len(), 2);
    /// }
    /// ```
    pub fn filter_label<'a, F>(&'a self, filter: F) -> Vec<&'a VirtualNode>
    where
        F: Fn(&str) -> bool,
    {
        // Get descendants recursively
        let mut descendants: Vec<&'a VirtualNode> = vec![];
        match self {
            VirtualNode::Text(_) => { /* nothing to do */ }
            VirtualNode::Element(element_node) => {
                for child in element_node.children.iter() {
                    get_descendants(&mut descendants, child);
                }
            }
        }

        // Filter descendants
        descendants
            .into_iter()
            .filter(|vn: &&'a VirtualNode| match vn {
                VirtualNode::Text(_) => false,
                VirtualNode::Element(element_node) => match element_node.attrs.get("label") {
                    Some(label) => filter(label),
                    None => false,
                },
            })
            .collect()
    }

    /// Get a vector of all of the descendants of this VirtualNode
    /// that have the provided `filter`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # #[macro_use] extern crate virtual_dom_rs;  fn main() {
    ///
    /// let component = html! {<div>
    ///  <span label="hello",> {"Hi!"} </span>
    ///  <em label="world",> {"There!!"} </em>
    ///  <em label="hello",></em>
    /// </div> };
    ///
    /// let hello_nodes = component.filter_label_equals("hello");
    ///
    /// assert_eq!(hello_nodes.len(), 2);
    /// }
    /// ```
    pub fn filter_label_equals<'a>(&'a self, label: &str) -> Vec<&'a VirtualNode> {
        self.filter_label(|node_label| node_label == label)
    }
}

fn get_descendants<'a>(descendants: &mut Vec<&'a VirtualNode>, node: &'a VirtualNode) {
    descendants.push(node);
    match node {
        VirtualNode::Text(_) => { /* nothing to do */ }
        VirtualNode::Element(element_node) => {
            for child in element_node.children.iter() {
                get_descendants(descendants, child);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VElement;
    use std::collections::HashMap;

    // TODO: Move this test somewhere that we can use the `html!` macro
    //    #[test]
    //    fn filter_label() {
    //        let html = html! {
    //        // Should not pick up labels on the root node
    //        <div label="hello0",>
    //            // This node gets picked up
    //            <span label="hello1",>
    //            </span>
    //            // This node gets picked up
    //            <em label="hello2",>
    //                { "hello there :)!" }
    //            </em>
    //            <div label="world",></div>
    //        </div>
    //        };
    //
    //        let hello_nodes = html.filter_label(|label| label.contains("hello"));
    //
    //        assert_eq!(
    //            hello_nodes.len(),
    //            2,
    //            "2 elements with label containing 'hello'"
    //        );
    //    }

    #[test]
    fn label_equals() {
        let span = VirtualNode::element("span");

        let mut attrs = HashMap::new();
        attrs.insert("label".to_string(), "hello".to_string());
        let mut em = VElement::new("em");
        em.attrs = attrs;

        let mut html = VElement::new("div");
        html.children.push(span);
        html.children.push(em.into());

        let html_node = VirtualNode::Element(html);
        let hello_nodes = html_node.filter_label_equals("hello");

        assert_eq!(hello_nodes.len(), 1);
    }
}

use dioxus_core::prelude::*;
use dioxus_web::WebsysRenderer;

fn main() {
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example));
}

fn Component(ctx: Context, props: ()) -> DomTree {
    let user_data = use_sql_query(&ctx, USER_DATA_QUERY);

    ctx.render(rsx! {
        h1 { "Hello, {username}"}
        button {
            "Delete user"
            onclick: move |_| user_data.delete()
        }
    })
}
