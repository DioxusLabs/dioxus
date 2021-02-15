//! Diff virtual-doms and patch the real DOM

use crate::diff::diff;
use crate::patch::patch;
use std::collections::HashMap;
use virtual_node::DynClosure;
use virtual_node::VirtualNode;
use web_sys::{Element, Node};

/// Closures that we are holding on to to make sure that they don't get invalidated after a
/// VirtualNode is dropped.
///
/// The u32 is a unique identifier that is associated with the DOM element that this closure is
/// attached to.
///
/// TODO: Periodically check if the DOM element is still there, and if not drop the closure.
///   Maybe whenever a DOM node is replaced or truncated we figure out all of it's
///   descendants somehow and invalidate those closures..? Need to plan this out..
///   At it stands now this hashmap will grow anytime a new element with closures is
///   appended or replaced and we will never free those closures.
pub type ActiveClosures = HashMap<u32, Vec<DynClosure>>;

/// Used for keeping a real DOM node up to date based on the current VirtualNode
/// and a new incoming VirtualNode that represents our latest DOM state.
pub struct DomUpdater {
    current_vdom: VirtualNode,
    /// The closures that are currently attached to elements in the page.
    ///
    /// We keep these around so that they don't get dropped (and thus stop working);
    ///
    /// FIXME: Drop them when the element is no longer in the page. Need to figure out
    /// a good strategy for when to do this.
    pub active_closures: ActiveClosures,
    root_node: Node,
}

impl DomUpdater {
    /// Create a new `DomUpdater`.
    ///
    /// A root `Node` will be created but not added to your DOM.
    pub fn new(current_vdom: VirtualNode) -> DomUpdater {
        let created_node = current_vdom.create_dom_node();
        DomUpdater {
            current_vdom,
            active_closures: created_node.closures,
            root_node: created_node.node,
        }
    }

    /// Create a new `DomUpdater`.
    ///
    /// A root `Node` will be created and appended (as a child) to your passed
    /// in mount element.
    pub fn new_append_to_mount(current_vdom: VirtualNode, mount: &Element) -> DomUpdater {
        let created_node = current_vdom.create_dom_node();
        mount
            .append_child(&created_node.node)
            .expect("Could not append child to mount");
        DomUpdater {
            current_vdom,
            active_closures: created_node.closures,
            root_node: created_node.node,
        }
    }

    /// Create a new `DomUpdater`.
    ///
    /// A root `Node` will be created and it will replace your passed in mount
    /// element.
    pub fn new_replace_mount(current_vdom: VirtualNode, mount: Element) -> DomUpdater {
        let created_node = current_vdom.create_dom_node();
        mount
            .replace_with_with_node_1(&created_node.node)
            .expect("Could not replace mount element");
        DomUpdater {
            current_vdom,
            active_closures: created_node.closures,
            root_node: created_node.node,
        }
    }

    /// Diff the current virtual dom with the new virtual dom that is being passed in.
    ///
    /// Then use that diff to patch the real DOM in the user's browser so that they are
    /// seeing the latest state of the application.
    pub fn update(&mut self, new_vdom: VirtualNode) {
        let patches = diff(&self.current_vdom, &new_vdom);

        let active_closures = patch(self.root_node.clone(), &patches).unwrap();

        self.active_closures.extend(active_closures);

        self.current_vdom = new_vdom;
    }

    /// Return the root node of your application, the highest ancestor of all other nodes in
    /// your real DOM tree.
    pub fn root_node(&self) -> Node {
        // Note that we're cloning the `web_sys::Node`, not the DOM element.
        // So we're effectively cloning a pointer here, which is fast.
        self.root_node.clone()
    }
}
