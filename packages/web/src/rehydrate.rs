use crate::dom::WebsysDom;
use dioxus_core::AttributeValue;
use dioxus_core::{DynamicNode, ElementId, ScopeState, TemplateNode, VNode, VirtualDom};

#[derive(Debug)]
pub enum RehydrationError {
    VNodeNotInitialized,
}

use RehydrationError::*;

impl WebsysDom {
    // we're streaming in patches, but the nodes already exist
    // so we're just going to write the correct IDs to the node and load them in
    pub fn rehydrate(&mut self, dom: &VirtualDom) -> Result<(), RehydrationError> {
        let root_scope = dom.base_scope();
        let mut ids = Vec::new();
        let mut to_mount = Vec::new();

        // Recursively rehydrate the dom from the VirtualDom
        self.rehydrate_scope(root_scope, dom, &mut ids, &mut to_mount)?;

        dioxus_interpreter_js::hydrate(ids);

        for id in to_mount {
            self.send_mount_event(id);
        }

        Ok(())
    }

    fn rehydrate_scope(
        &mut self,
        scope: &ScopeState,
        dom: &VirtualDom,
        ids: &mut Vec<u32>,
        to_mount: &mut Vec<ElementId>,
    ) -> Result<(), RehydrationError> {
        let vnode = match scope.root_node() {
            dioxus_core::RenderReturn::Ready(ready) => ready,
            _ => return Err(VNodeNotInitialized),
        };
        self.rehydrate_vnode(dom, vnode, ids, to_mount)
    }

    fn rehydrate_vnode(
        &mut self,
        dom: &VirtualDom,
        vnode: &VNode,
        ids: &mut Vec<u32>,
        to_mount: &mut Vec<ElementId>,
    ) -> Result<(), RehydrationError> {
        for (i, root) in vnode.template.get().roots.iter().enumerate() {
            self.rehydrate_template_node(
                dom,
                vnode,
                root,
                ids,
                to_mount,
                Some(*vnode.root_ids.borrow().get(i).ok_or(VNodeNotInitialized)?),
            )?;
        }
        Ok(())
    }

    fn rehydrate_template_node(
        &mut self,
        dom: &VirtualDom,
        vnode: &VNode,
        node: &TemplateNode,
        ids: &mut Vec<u32>,
        to_mount: &mut Vec<ElementId>,
        root_id: Option<ElementId>,
    ) -> Result<(), RehydrationError> {
        tracing::trace!("rehydrate template node: {:?}", node);
        match node {
            TemplateNode::Element {
                children, attrs, ..
            } => {
                let mut mounted_id = root_id;
                for attr in *attrs {
                    if let dioxus_core::TemplateAttribute::Dynamic { id } = attr {
                        let attribute = &vnode.dynamic_attrs[*id];
                        let id = attribute.mounted_element();
                        attribute.attribute_type().for_each(|attribute| {
                            let value = &attribute.value;
                            mounted_id = Some(id);
                            if let AttributeValue::Listener(_) = value {
                                if attribute.name == "onmounted" {
                                    to_mount.push(id);
                                }
                            }
                        });
                    }
                }
                if let Some(id) = mounted_id {
                    ids.push(id.0 as u32);
                }
                if !children.is_empty() {
                    for child in *children {
                        self.rehydrate_template_node(dom, vnode, child, ids, to_mount, None)?;
                    }
                }
            }
            TemplateNode::Dynamic { id } | TemplateNode::DynamicText { id } => {
                self.rehydrate_dynamic_node(dom, &vnode.dynamic_nodes[*id], ids, to_mount)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn rehydrate_dynamic_node(
        &mut self,
        dom: &VirtualDom,
        dynamic: &DynamicNode,
        ids: &mut Vec<u32>,
        to_mount: &mut Vec<ElementId>,
    ) -> Result<(), RehydrationError> {
        tracing::trace!("rehydrate dynamic node: {:?}", dynamic);
        match dynamic {
            dioxus_core::DynamicNode::Text(text) => {
                ids.push(text.mounted_element().ok_or(VNodeNotInitialized)?.0 as u32);
            }
            dioxus_core::DynamicNode::Placeholder(placeholder) => {
                ids.push(placeholder.mounted_element().ok_or(VNodeNotInitialized)?.0 as u32);
            }
            dioxus_core::DynamicNode::Component(comp) => {
                let scope = comp.mounted_scope().ok_or(VNodeNotInitialized)?;
                self.rehydrate_scope(
                    dom.get_scope(scope).ok_or(VNodeNotInitialized)?,
                    dom,
                    ids,
                    to_mount,
                )?;
            }
            dioxus_core::DynamicNode::Fragment(fragment) => {
                for vnode in *fragment {
                    self.rehydrate_vnode(dom, vnode, ids, to_mount)?;
                }
            }
        }
        Ok(())
    }
}
