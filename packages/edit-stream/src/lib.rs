use dioxus_core::*;

// /// ## Mutations
// ///
// /// This method returns "mutations" - IE the necessary changes to get the RealDOM to match the VirtualDOM. It also
// /// includes a list of NodeRefs that need to be applied and effects that need to be triggered after the RealDOM has
// /// applied the edits.
// ///
// /// Mutations are the only link between the RealDOM and the VirtualDOM.
// pub struct Mutations<'a> {
//     /// The list of edits that need to be applied for the RealDOM to match the VirtualDOM.
//     pub edits: Vec<DomEdit<'a>>,

//     /// The list of Scopes that were diffed, created, and removed during the Diff process.
//     pub dirty_scopes: FxHashSet<ScopeId>,

//     /// The list of nodes to connect to the RealDOM.
//     pub refs: Vec<NodeRefMutation<'a>>,
// }

// impl Debug for Mutations<'_> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("Mutations")
//             .field("edits", &self.edits)
//             .field("noderefs", &self.refs)
//             .finish()
//     }
// }

// /// A `DomEdit` represents a serialized form of the VirtualDom's trait-based API. This allows streaming edits across the
// /// network or through FFI boundaries.
// #[derive(Debug, PartialEq)]
// #[cfg_attr(
//     feature = "serialize",
//     derive(serde::Serialize, serde::Deserialize),
//     serde(tag = "type")
// )]
// pub enum DomEdit<'bump> {
//     /// Push the given root node onto our stack.
//     PushRoot {
//         /// The ID of the root node to push.
//         root: ElementId,
//     },

//     /// Pop the topmost node from our stack and append them to the node
//     /// at the top of the stack.
//     AppendChildren {
//         /// How many nodes should be popped from the stack.
//         /// The node remaining on the stack will be the target for the append.
//         many: u32,
//     },

//     /// Replace a given (single) node with a handful of nodes currently on the stack.
//     ReplaceWith {
//         /// The ID of the node to be replaced.
//         root: ElementId,

//         /// How many nodes should be popped from the stack to replace the target node.
//         m: u32,
//     },

//     /// Insert a number of nodes after a given node.
//     InsertAfter {
//         /// The ID of the node to insert after.
//         root: ElementId,

//         /// How many nodes should be popped from the stack to insert after the target node.
//         n: u32,
//     },

//     /// Insert a number of nodes before a given node.
//     InsertBefore {
//         /// The ID of the node to insert before.
//         root: ElementId,

//         /// How many nodes should be popped from the stack to insert before the target node.
//         n: u32,
//     },

//     /// Remove a particular node from the DOM
//     Remove {
//         /// The ID of the node to remove.
//         root: ElementId,
//     },

//     /// Create a new purely-text node
//     CreateTextNode {
//         /// The ID the new node should have.
//         root: ElementId,

//         /// The textcontent of the node
//         text: &'bump str,
//     },

//     /// Create a new purely-element node
//     CreateElement {
//         /// The ID the new node should have.
//         root: ElementId,

//         /// The tagname of the node
//         tag: &'bump str,
//     },

//     /// Create a new purely-comment node with a given namespace
//     CreateElementNs {
//         /// The ID the new node should have.
//         root: ElementId,

//         /// The namespace of the node
//         tag: &'bump str,

//         /// The namespace of the node (like `SVG`)
//         ns: &'static str,
//     },

//     /// Create a new placeholder node.
//     /// In most implementations, this will either be a hidden div or a comment node.
//     CreatePlaceholder {
//         /// The ID the new node should have.
//         root: ElementId,
//     },

//     /// Create a new Event Listener.
//     NewEventListener {
//         /// The name of the event to listen for.
//         event_name: &'static str,

//         /// The ID of the node to attach the listener to.
//         scope: ScopeId,

//         /// The ID of the node to attach the listener to.
//         root: ElementId,
//     },

//     /// Remove an existing Event Listener.
//     RemoveEventListener {
//         /// The ID of the node to remove.
//         root: ElementId,

//         /// The name of the event to remove.
//         event: &'static str,
//     },

//     /// Set the textcontent of a node.
//     SetText {
//         /// The ID of the node to set the textcontent of.
//         root: ElementId,

//         /// The textcontent of the node
//         text: &'bump str,
//     },

//     /// Set the value of a node's attribute.
//     SetAttribute {
//         /// The ID of the node to set the attribute of.
//         root: ElementId,

//         /// The name of the attribute to set.
//         field: &'static str,

//         /// The value of the attribute.
//         value: AttributeValue<'bump>,

//         // value: &'bump str,
//         /// The (optional) namespace of the attribute.
//         /// For instance, "style" is in the "style" namespace.
//         ns: Option<&'bump str>,
//     },

//     /// Remove an attribute from a node.
//     RemoveAttribute {
//         /// The ID of the node to remove.
//         root: ElementId,

//         /// The name of the attribute to remove.
//         name: &'static str,

//         /// The namespace of the attribute.
//         ns: Option<&'bump str>,
//     },

//     /// Manually pop a root node from the stack.
//     PopRoot {
//         /// The amount of nodes to pop
//         count: u32,
//     },

//     /// Remove all the children of an element
//     RemoveChildren {
//         /// The root
//         root: ElementId,
//     },

//     /// Create a template using the nodes on the stack
//     StoreTemplate {
//         /// The ID of the template
//         name: &'static str,

//         /// The amount of nodes to pop from the stack into the template
//         num_children: u32,

//         /// Indicies for the nodes to pop from the stack into the template
//         dynamic_nodes: &'static [&'static [u32]],
//     },

//     /// Load the template onto the stack
//     LoadTemplate {
//         /// The ID of the template
//         name: &'static str,

//         /// The index of the template body
//         index: u32,

//         /// Give the node a new ID to remove it later
//         root: ElementId,
//     },

//     /// Load n nodes into the kth dynamic node of the template
//     ///
//     /// Assumes that the template is already on the stack
//     MergeTemplate {
//         /// The index of the dynamic node to merge into
//         index: u32,

//         /// The amount of nodes to pop from the stack into the template
//         num_children: u32,
//     },
// }

// use DomEdit::*;

// impl<'a> Mutations<'a> {
//     pub(crate) fn new() -> Self {
//         Self {
//             edits: Vec::new(),
//             refs: Vec::new(),
//             dirty_scopes: Default::default(),
//         }
//     }

//     // Navigation
//     pub(crate) fn push_root(&mut self, root: ElementId) {
//         self.edits.push(PushRoot { root });
//     }

//     // Navigation
//     pub(crate) fn pop_root(&mut self) {
//         self.edits.push(PopRoot { count: 1 });
//     }

//     pub(crate) fn replace_with(&mut self, root: ElementId, m: u32) {
//         self.edits.push(ReplaceWith { m, root });
//     }

//     pub(crate) fn insert_after(&mut self, root: ElementId, n: u32) {
//         self.edits.push(InsertAfter { n, root });
//     }

//     pub(crate) fn insert_before(&mut self, root: ElementId, n: u32) {
//         self.edits.push(InsertBefore { n, root });
//     }

//     pub(crate) fn append_children(&mut self, n: u32) {
//         self.edits.push(AppendChildren { many: n });
//     }

//     // Remove Nodes from the dom
//     pub(crate) fn remove(&mut self, root: ElementId) {
//         self.edits.push(Remove { root });
//     }

//     // Create
//     pub(crate) fn create_text_node(&mut self, text: &'a str, root: ElementId) {
//         self.edits.push(CreateTextNode { text, root });
//     }

//     pub(crate) fn create_element(
//         &mut self,
//         tag: &'static str,
//         ns: Option<&'static str>,
//         id: ElementId,
//     ) {
//         match ns {
//             Some(ns) => self.edits.push(CreateElementNs { root: id, ns, tag }),
//             None => self.edits.push(CreateElement { root: id, tag }),
//         }
//     }
//     // placeholders are nodes that don't get rendered but still exist as an "anchor" in the real dom
//     pub(crate) fn create_placeholder(&mut self, id: ElementId) {
//         self.edits.push(CreatePlaceholder { root: id });
//     }

//     // events
//     pub(crate) fn new_event_listener(&mut self, listener: &Listener, scope: ScopeId) {
//         let Listener {
//             event,
//             mounted_node,
//             ..
//         } = listener;

//         let element_id = mounted_node.get().unwrap();

//         self.edits.push(NewEventListener {
//             scope,
//             event_name: event,
//             root: element_id,
//         });
//     }
//     pub(crate) fn remove_event_listener(&mut self, event: &'static str, root: ElementId) {
//         self.edits.push(RemoveEventListener { event, root });
//     }

//     // modify
//     pub(crate) fn set_text(&mut self, text: &'a str, root: ElementId) {
//         self.edits.push(SetText { text, root });
//     }

//     pub(crate) fn set_attribute(&mut self, attribute: &'a Attribute<'a>, root: ElementId) {
//         let Attribute {
//             name,
//             value,
//             namespace,
//             ..
//         } = attribute;

//         self.edits.push(SetAttribute {
//             field: name,
//             value: value.clone(),
//             ns: *namespace,
//             root,
//         });
//     }

//     pub(crate) fn remove_attribute(&mut self, attribute: &Attribute, root: ElementId) {
//         let Attribute {
//             name, namespace, ..
//         } = attribute;

//         self.edits.push(RemoveAttribute {
//             name,
//             ns: *namespace,
//             root,
//         });
//     }

//     pub(crate) fn mark_dirty_scope(&mut self, scope: ScopeId) {
//         self.dirty_scopes.insert(scope);
//     }
// }

// // refs are only assigned once
// pub struct NodeRefMutation<'a> {
//     pub element: &'a mut Option<once_cell::sync::OnceCell<Box<dyn Any>>>,
//     pub element_id: ElementId,
// }

// impl<'a> std::fmt::Debug for NodeRefMutation<'a> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("NodeRefMutation")
//             .field("element_id", &self.element_id)
//             .finish()
//     }
// }

// impl<'a> NodeRefMutation<'a> {
//     pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
//         self.element
//             .as_ref()
//             .and_then(|f| f.get())
//             .and_then(|f| f.downcast_ref::<T>())
//     }
//     pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
//         self.element
//             .as_mut()
//             .and_then(|f| f.get_mut())
//             .and_then(|f| f.downcast_mut::<T>())
//     }
// }
