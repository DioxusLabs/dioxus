use std::{collections::HashMap, fmt, rc::Rc};
use web_sys::{self, Element, EventTarget, Node, Text};

use dioxus_core::prelude::{VElement, VNode, VText, VirtualNode};
use std::ops::Deref;
use std::sync::Mutex;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

pub struct DomRenderer {}

// Used to uniquely identify elements that contain closures so that the DomUpdater can
// look them up by their unique id.
// When the DomUpdater sees that the element no longer exists it will drop all of it's
// Rc'd Closures for those events.
use lazy_static::lazy_static;
lazy_static! {
    static ref ELEM_UNIQUE_ID: Mutex<u32> = Mutex::new(0);
}

fn create_unique_identifier() -> u32 {
    let mut elem_unique_id = ELEM_UNIQUE_ID.lock().unwrap();
    *elem_unique_id += 1;
    *elem_unique_id
}

/// A node along with all of the closures that were created for that
/// node's events and all of it's child node's events.
pub struct CreatedNode<T> {
    /// A `Node` or `Element` that was created from a `VirtualNode`
    pub node: T,
    /// A map of a node's unique identifier along with all of the Closures for that node.
    ///
    /// The DomUpdater uses this to look up nodes and see if they're still in the page. If not
    /// the reference that we maintain to their closure will be dropped, thus freeing the Closure's
    /// memory.
    pub closures: HashMap<u32, Vec<DynClosure>>,
}

impl<T> CreatedNode<T> {
    pub fn without_closures<N: Into<T>>(node: N) -> Self {
        CreatedNode {
            node: node.into(),
            closures: HashMap::with_capacity(0),
        }
    }
}

impl<T> Deref for CreatedNode<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl From<CreatedNode<Element>> for CreatedNode<Node> {
    fn from(other: CreatedNode<Element>) -> CreatedNode<Node> {
        CreatedNode {
            node: other.node.into(),
            closures: other.closures,
        }
    }
}

//----------------------------------
// Create nodes for the VNode types
// ---------------------------------

/// Return a `Text` element from a `VirtualNode`, typically right before adding it
/// into the DOM.
pub fn create_text_node(text_node: &VText) -> Text {
    let document = web_sys::window().unwrap().document().unwrap();
    document.create_text_node(&text_node.text)
}

/// Build a DOM element by recursively creating DOM nodes for this element and it's
/// children, it's children's children, etc.
pub fn create_element_node(velement: &VElement) -> CreatedNode<Element> {
    let document = web_sys::window().unwrap().document().unwrap();

    let element = if html_validation::is_svg_namespace(&velement.tag) {
        document
            .create_element_ns(Some("http://www.w3.org/2000/svg"), &velement.tag)
            .unwrap()
    } else {
        document.create_element(&velement.tag).unwrap()
    };

    let mut closures = HashMap::new();

    velement.attrs.iter().for_each(|(name, value)| {
        if name == "unsafe_inner_html" {
            element.set_inner_html(value);

            return;
        }

        element
            .set_attribute(name, value)
            .expect("Set element attribute in create element");
    });

    todo!("Support events properly in web ");
    // if velement.events.0.len() > 0 {
    //     let unique_id = create_unique_identifier();

    //     element
    //         .set_attribute("data-vdom-id".into(), &unique_id.to_string())
    //         .expect("Could not set attribute on element");

    //     closures.insert(unique_id, vec![]);

    //     velement.events.0.iter().for_each(|(onevent, callback)| {
    //         // onclick -> click
    //         let event = &onevent[2..];

    //         let current_elem: &EventTarget = element.dyn_ref().unwrap();

    //         current_elem
    //             .add_event_listener_with_callback(event, callback.as_ref().as_ref().unchecked_ref())
    //             .unwrap();

    //         closures
    //             .get_mut(&unique_id)
    //             .unwrap()
    //             .push(Rc::clone(callback));
    //     });
    // }

    let mut previous_node_was_text = false;

    velement.children.iter().for_each(|child| {
        match child {
            VNode::Text(text_node) => {
                let current_node = element.as_ref() as &web_sys::Node;

                // We ensure that the text siblings are patched by preventing the browser from merging
                // neighboring text nodes. Originally inspired by some of React's work from 2016.
                //  -> https://reactjs.org/blog/2016/04/07/react-v15.html#major-changes
                //  -> https://github.com/facebook/react/pull/5753
                //
                // `ptns` = Percy text node separator
                if previous_node_was_text {
                    let separator = document.create_comment("ptns");
                    current_node
                        .append_child(separator.as_ref() as &web_sys::Node)
                        .unwrap();
                }

                current_node
                    .append_child(&create_text_node(&text_node))
                    // .append_child(&text_node.create_text_node())
                    .unwrap();

                previous_node_was_text = true;
            }
            VNode::Element(element_node) => {
                previous_node_was_text = false;

                let child = create_element_node(&element_node);
                // let child = element_node.create_element_node();
                let child_elem: Element = child.node;

                closures.extend(child.closures);

                element.append_child(&child_elem).unwrap();
            }

            VNode::Component(component) => {
                //
                todo!("Support components in the web properly");
            }
        }
    });

    todo!("Support events properly in web ");
    // if let Some(on_create_elem) = velement.events.0.get("on_create_elem") {
    //     let on_create_elem: &js_sys::Function = on_create_elem.as_ref().as_ref().unchecked_ref();
    //     on_create_elem
    //         .call1(&wasm_bindgen::JsValue::NULL, &element)
    //         .unwrap();
    // }

    CreatedNode {
        node: element,
        closures,
    }
}

/// Box<dyn AsRef<JsValue>>> is our js_sys::Closure. Stored this way to allow us to store
/// any Closure regardless of the arguments.
pub type DynClosure = Rc<dyn AsRef<JsValue>>;

/// We need a custom implementation of fmt::Debug since JsValue doesn't
/// implement debug.
pub struct Events(pub HashMap<String, DynClosure>);

impl PartialEq for Events {
    // TODO: What should happen here..? And why?
    fn eq(&self, _rhs: &Self) -> bool {
        true
    }
}

impl fmt::Debug for Events {
    // Print out all of the event names for this VirtualNode
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let events: String = self.0.keys().map(|key| " ".to_string() + key).collect();
        write!(f, "{}", events)
    }
}
