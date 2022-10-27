use dioxus_core::*;

/// ## Mutations
///
/// This method returns "mutations" - IE the necessary changes to get the RealDOM to match the VirtualDOM. It also
/// includes a list of NodeRefs that need to be applied and effects that need to be triggered after the RealDOM has
/// applied the edits.
///
/// Mutations are the only link between the RealDOM and the VirtualDOM.
#[derive(Default)]
pub struct Mutations<'a> {
    /// The list of edits that need to be applied for the RealDOM to match the VirtualDOM.
    pub edits: Vec<DomEdit<'a>>,

    /// The list of Scopes that were diffed, created, and removed during the Diff process.
    pub dirty_scopes: Vec<ScopeId>,
}

/// A `DomEdit` represents a serialized form of the VirtualDom's trait-based API. This allows streaming edits across the
/// network or through FFI boundaries.
#[derive(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type")
)]
pub enum DomEdit<'bump> {
    /// Push the given root node onto our stack.
    PushRoot {
        /// The ID of the root node to push.
        root: ElementId,
    },

    /// Pop the topmost node from our stack and append them to the node
    /// at the top of the stack.
    AppendChildren {
        /// How many nodes should be popped from the stack.
        /// The node remaining on the stack will be the target for the append.
        many: u32,
    },

    /// Replace a given (single) node with a handful of nodes currently on the stack.
    ReplaceWith {
        /// The ID of the node to be replaced.
        root: ElementId,

        /// How many nodes should be popped from the stack to replace the target node.
        m: u32,
    },

    /// Insert a number of nodes after a given node.
    InsertAfter {
        /// The ID of the node to insert after.
        root: ElementId,

        /// How many nodes should be popped from the stack to insert after the target node.
        n: u32,
    },

    /// Insert a number of nodes before a given node.
    InsertBefore {
        /// The ID of the node to insert before.
        root: ElementId,

        /// How many nodes should be popped from the stack to insert before the target node.
        n: u32,
    },

    /// Remove a particular node from the DOM
    Remove {
        /// The ID of the node to remove.
        root: ElementId,
    },

    /// Create a new purely-text node
    CreateTextNode {
        /// The ID the new node should have.
        root: ElementId,

        /// The textcontent of the node
        text: &'bump str,
    },

    /// Create a new purely-element node
    CreateElement {
        /// The ID the new node should have.
        root: ElementId,

        /// The tagname of the node
        tag: &'bump str,
    },

    /// Create a new purely-comment node with a given namespace
    CreateElementNs {
        /// The ID the new node should have.
        root: ElementId,

        /// The namespace of the node
        tag: &'bump str,

        /// The namespace of the node (like `SVG`)
        ns: &'static str,
    },

    /// Create a new placeholder node.
    /// In most implementations, this will either be a hidden div or a comment node.
    CreatePlaceholder {
        /// The ID the new node should have.
        root: ElementId,
    },

    /// Create a new Event Listener.
    NewEventListener {
        /// The name of the event to listen for.
        event_name: &'static str,

        /// The ID of the node to attach the listener to.
        scope: ScopeId,

        /// The ID of the node to attach the listener to.
        root: ElementId,
    },

    /// Remove an existing Event Listener.
    RemoveEventListener {
        /// The ID of the node to remove.
        root: ElementId,

        /// The name of the event to remove.
        event: &'static str,
    },

    /// Set the textcontent of a node.
    SetText {
        /// The ID of the node to set the textcontent of.
        root: ElementId,

        /// The textcontent of the node
        text: &'bump str,
    },

    /// Set the value of a node's attribute.
    SetAttribute {
        /// The ID of the node to set the attribute of.
        root: ElementId,

        /// The name of the attribute to set.
        field: &'static str,

        /// The value of the attribute.
        value: AttributeValue<'bump>,

        // value: &'bump str,
        /// The (optional) namespace of the attribute.
        /// For instance, "style" is in the "style" namespace.
        ns: Option<&'bump str>,
    },

    /// Remove an attribute from a node.
    RemoveAttribute {
        /// The ID of the node to remove.
        root: ElementId,

        /// The name of the attribute to remove.
        name: &'static str,

        /// The namespace of the attribute.
        ns: Option<&'bump str>,
    },

    /// Manually pop a root node from the stack.
    PopRoot {
        /// The amount of nodes to pop
        count: u32,
    },

    /// Remove all the children of an element
    RemoveChildren {
        /// The root
        root: ElementId,
    },

    /*

    Template stuff

    - load into scratch space
    - dump nodes into stack
    - assign ids of nodes in template

    */
    /// Create a template using the nodes on the stack
    Save {
        /// The ID of the template
        name: &'static str,

        /// The amount of nodes to pop from the stack into the template
        num_children: u32,
    },

    /// Load the template into a scratch space on the stack
    ///
    /// The template body now lives on the stack, but needs to be finished before its nodes can be appended to the DOM.
    Load {
        /// The ID of the template
        name: &'static str,

        id: u32,
    },

    AssignId {
        index: &'static str,
        id: ElementId,
    },

    ReplaceDescendant {
        index: &'static str,
        m: u32,
    },
}

use DomEdit::*;

impl<'a> dioxus_core::Renderer<'a> for Mutations<'a> {
    // Navigation
    fn push_root(&mut self, root: ElementId) {
        self.edits.push(PushRoot { root });
    }

    // Navigation
    fn pop_root(&mut self) {
        self.edits.push(PopRoot { count: 1 });
    }

    fn replace_with(&mut self, root: ElementId, m: u32) {
        self.edits.push(ReplaceWith { m, root });
    }

    fn replace_descendant(&mut self, descendent: &'static [u8], m: u32) {
        self.edits.push(ReplaceDescendant {
            // serializing is just hijacking ascii
            index: unsafe { std::str::from_utf8_unchecked(descendent) },
            m,
        });
    }

    fn insert_after(&mut self, root: ElementId, n: u32) {
        self.edits.push(InsertAfter { n, root });
    }

    fn insert_before(&mut self, root: ElementId, n: u32) {
        self.edits.push(InsertBefore { n, root });
    }

    fn append_children(&mut self, n: u32) {
        self.edits.push(AppendChildren { many: n });
    }

    // Create
    fn create_text_node(&mut self, text: &'a str, root: ElementId) {
        self.edits.push(CreateTextNode { text, root });
    }

    fn create_element(&mut self, tag: &'static str, ns: Option<&'static str>, id: ElementId) {
        match ns {
            Some(ns) => self.edits.push(CreateElementNs { root: id, ns, tag }),
            None => self.edits.push(CreateElement { root: id, tag }),
        }
    }

    // placeholders are nodes that don't get rendered but still exist as an "anchor" in the real dom
    fn create_placeholder(&mut self, id: ElementId) {
        self.edits.push(CreatePlaceholder { root: id });
    }

    fn assign_id(&mut self, descendent: &'static [u8], id: ElementId) {
        self.edits.push(AssignId {
            index: unsafe { std::str::from_utf8_unchecked(descendent) },
            id,
        });
    }

    // Remove Nodes from the dom
    fn remove(&mut self, root: ElementId) {
        self.edits.push(Remove { root });
    }

    fn remove_attribute(&mut self, attribute: &Attribute, root: ElementId) {
        self.edits.push(RemoveAttribute {
            name: attribute.name,
            ns: attribute.namespace,
            root,
        });
    }

    // events
    fn new_event_listener(&mut self, listener: &Listener, scope: ScopeId) {
        let Listener {
            event,
            mounted_node,
            ..
        } = listener;

        let element_id = mounted_node.get();

        self.edits.push(NewEventListener {
            scope,
            event_name: event,
            root: element_id,
        });
    }

    fn remove_event_listener(&mut self, event: &'static str, root: ElementId) {
        self.edits.push(RemoveEventListener { event, root });
    }

    // modify
    fn set_text(&mut self, text: &'a str, root: ElementId) {
        self.edits.push(SetText { text, root });
    }

    fn save(&mut self, id: &'static str, num: u32) {
        self.edits.push(Save {
            name: id,
            num_children: num,
        });
    }

    fn load(&mut self, id: &'static str, index: u32) {
        self.edits.push(Load {
            name: id,
            id: index,
        });
    }

    fn mark_dirty_scope(&mut self, scope: ScopeId) {
        self.dirty_scopes.push(scope);
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        value: AttributeValue<'a>,
        namespace: Option<&'a str>,
        root: ElementId,
    ) {
        self.edits.push(SetAttribute {
            field: name,
            value: value.clone(),
            ns: namespace,
            root,
        });
    }

    fn remove_children(&mut self, element: ElementId) {
        todo!()
    }
}
