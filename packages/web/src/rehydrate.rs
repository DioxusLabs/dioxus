use crate::dom::WebsysDom;
use dioxus_core::{
    AttributeValue, DynamicNode, ElementId, ScopeState, TemplateNode, VNode, VirtualDom,
};
use dioxus_html::event_bubbles;
use wasm_bindgen::JsCast;
use web_sys::{Comment, Node};

#[derive(Debug, Copy, Clone)]
pub enum RehydrationError {
    NodeTypeMismatch,
    NodeNotFound,
    VNodeNotInitialized,
}
use RehydrationError::*;

fn set_node(hydrated: &mut Vec<bool>, id: ElementId, node: Node) {
    let idx = id.0;
    if idx >= hydrated.len() {
        hydrated.resize(idx + 1, false);
    }
    if !hydrated[idx] {
        dioxus_interpreter_js::set_node(idx as u32, node);
        hydrated[idx] = true;
    }
}

impl WebsysDom {
    // we're streaming in patches, but the nodes already exist
    // so we're just going to write the correct IDs to the node and load them in
    pub fn rehydrate(&mut self, dom: &VirtualDom) -> Result<(), RehydrationError> {
        let mut root = self
            .root
            .clone()
            .dyn_into::<Node>()
            .map_err(|_| NodeTypeMismatch)?
            .first_child()
            .ok_or(NodeNotFound);

        let root_scope = dom.base_scope();

        let mut hydrated = vec![true];

        let mut last_node_was_static_text = false;

        // Recursively rehydrate the dom from the VirtualDom
        self.rehydrate_scope(
            root_scope,
            &mut root,
            &mut hydrated,
            dom,
            &mut last_node_was_static_text,
        )?;

        self.interpreter.flush();
        Ok(())
    }

    fn rehydrate_scope(
        &mut self,
        scope: &ScopeState,
        current_child: &mut Result<Node, RehydrationError>,
        hydrated: &mut Vec<bool>,
        dom: &VirtualDom,
        last_node_was_static_text: &mut bool,
    ) -> Result<(), RehydrationError> {
        let vnode = match scope.root_node() {
            dioxus_core::RenderReturn::Ready(ready) => ready,
            _ => return Err(VNodeNotInitialized),
        };
        self.rehydrate_vnode(
            current_child,
            hydrated,
            dom,
            vnode,
            last_node_was_static_text,
        )
    }

    fn rehydrate_vnode(
        &mut self,
        current_child: &mut Result<Node, RehydrationError>,
        hydrated: &mut Vec<bool>,
        dom: &VirtualDom,
        vnode: &VNode,
        last_node_was_static_text: &mut bool,
    ) -> Result<(), RehydrationError> {
        for (i, root) in vnode.template.get().roots.iter().enumerate() {
            // make sure we set the root node ids even if the node is not dynamic
            set_node(
                hydrated,
                *vnode.root_ids.borrow().get(i).ok_or(VNodeNotInitialized)?,
                current_child.clone()?,
            );

            self.rehydrate_template_node(
                current_child,
                hydrated,
                dom,
                vnode,
                root,
                last_node_was_static_text,
            )?;
        }
        Ok(())
    }

    fn rehydrate_template_node(
        &mut self,
        current_child: &mut Result<Node, RehydrationError>,
        hydrated: &mut Vec<bool>,
        dom: &VirtualDom,
        vnode: &VNode,
        node: &TemplateNode,
        last_node_was_static_text: &mut bool,
    ) -> Result<(), RehydrationError> {
        tracing::trace!("rehydrate template node: {:?}", node);
        if let Ok(current_child) = current_child {
            if tracing::event_enabled!(tracing::Level::TRACE) {
                web_sys::console::log_1(&current_child.clone().into());
            }
        }
        match node {
            TemplateNode::Element {
                children, attrs, ..
            } => {
                let mut mounted_id = None;
                for attr in *attrs {
                    if let dioxus_core::TemplateAttribute::Dynamic { id } = attr {
                        let attribute = &vnode.dynamic_attrs[*id];
                        let value = &attribute.value;
                        let id = attribute.mounted_element();
                        mounted_id = Some(id);
                        let name = attribute.name;
                        if let AttributeValue::Listener(_) = value {
                            let event_name = &name[2..];
                            self.interpreter.new_event_listener(
                                event_name,
                                id.0 as u32,
                                event_bubbles(event_name) as u8,
                            );
                        }
                    }
                }
                if let Some(id) = mounted_id {
                    set_node(hydrated, id, current_child.clone()?);
                }
                if !children.is_empty() {
                    let mut children_current_child = current_child
                        .as_mut()
                        .map_err(|e| *e)?
                        .first_child()
                        .ok_or(NodeNotFound)?
                        .dyn_into::<Node>()
                        .map_err(|_| NodeTypeMismatch);
                    for child in *children {
                        self.rehydrate_template_node(
                            &mut children_current_child,
                            hydrated,
                            dom,
                            vnode,
                            child,
                            last_node_was_static_text,
                        )?;
                    }
                }
                *current_child = current_child
                    .as_mut()
                    .map_err(|e| *e)?
                    .next_sibling()
                    .ok_or(NodeNotFound);
                *last_node_was_static_text = false;
            }
            TemplateNode::Text { .. } => {
                // if the last node was static text, it got merged with this one
                if !*last_node_was_static_text {
                    *current_child = current_child
                        .as_mut()
                        .map_err(|e| *e)?
                        .next_sibling()
                        .ok_or(NodeNotFound);
                }
                *last_node_was_static_text = true;
            }
            TemplateNode::Dynamic { id } | TemplateNode::DynamicText { id } => {
                self.rehydrate_dynamic_node(
                    current_child,
                    hydrated,
                    dom,
                    &vnode.dynamic_nodes[*id],
                    last_node_was_static_text,
                )?;
            }
        }
        Ok(())
    }

    fn rehydrate_dynamic_node(
        &mut self,
        current_child: &mut Result<Node, RehydrationError>,
        hydrated: &mut Vec<bool>,
        dom: &VirtualDom,
        dynamic: &DynamicNode,
        last_node_was_static_text: &mut bool,
    ) -> Result<(), RehydrationError> {
        tracing::trace!("rehydrate dynamic node: {:?}", dynamic);
        if let Ok(current_child) = current_child {
            if tracing::event_enabled!(tracing::Level::TRACE) {
                web_sys::console::log_1(&current_child.clone().into());
            }
        }
        match dynamic {
            dioxus_core::DynamicNode::Text(text) => {
                let id = text.mounted_element();
                // skip comment separator before node
                if cfg!(debug_assertions) {
                    assert!(current_child
                        .as_mut()
                        .map_err(|e| *e)?
                        .has_type::<Comment>());
                }
                *current_child = current_child
                    .as_mut()
                    .map_err(|e| *e)?
                    .next_sibling()
                    .ok_or(NodeNotFound);

                set_node(
                    hydrated,
                    id.ok_or(VNodeNotInitialized)?,
                    current_child.clone()?,
                );
                *current_child = current_child
                    .as_mut()
                    .map_err(|e| *e)?
                    .next_sibling()
                    .ok_or(NodeNotFound);

                // skip comment separator after node
                if cfg!(debug_assertions) {
                    assert!(current_child
                        .as_mut()
                        .map_err(|e| *e)?
                        .has_type::<Comment>());
                }
                *current_child = current_child
                    .as_mut()
                    .map_err(|e| *e)?
                    .next_sibling()
                    .ok_or(NodeNotFound);

                *last_node_was_static_text = false;
            }
            dioxus_core::DynamicNode::Placeholder(placeholder) => {
                set_node(
                    hydrated,
                    placeholder.mounted_element().ok_or(VNodeNotInitialized)?,
                    current_child.clone()?,
                );
                *current_child = current_child
                    .as_mut()
                    .map_err(|e| *e)?
                    .next_sibling()
                    .ok_or(NodeNotFound);
                *last_node_was_static_text = false;
            }
            dioxus_core::DynamicNode::Component(comp) => {
                let scope = comp.mounted_scope().ok_or(VNodeNotInitialized)?;
                self.rehydrate_scope(
                    dom.get_scope(scope).unwrap(),
                    current_child,
                    hydrated,
                    dom,
                    last_node_was_static_text,
                )?;
            }
            dioxus_core::DynamicNode::Fragment(fragment) => {
                for vnode in *fragment {
                    self.rehydrate_vnode(
                        current_child,
                        hydrated,
                        dom,
                        vnode,
                        last_node_was_static_text,
                    )?;
                }
            }
        }
        Ok(())
    }
}
