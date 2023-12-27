use crate::dom::WebsysDom;
use dioxus_core::{
     DynamicNode, ElementId, ScopeState, TemplateNode, VNode, VirtualDom,
};

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

        // Recursively rehydrate the dom from the VirtualDom
        self.rehydrate_scope(root_scope, dom, &mut ids)?;

        dioxus_interpreter_js::hydrate(ids);

        Ok(())
    }

    fn rehydrate_scope(
        &mut self,
        scope: &ScopeState,
        dom: &VirtualDom,
        ids: &mut Vec<u32>,
    ) -> Result<(), RehydrationError> {
        let vnode = match scope.root_node() {
            dioxus_core::RenderReturn::Ready(ready) => ready,
            _ => return Err(VNodeNotInitialized),
        };
        self.rehydrate_vnode(dom, vnode, ids)
    }

    fn rehydrate_vnode(
        &mut self,
        dom: &VirtualDom,
        vnode: &VNode,
        ids: &mut Vec<u32>,
    ) -> Result<(), RehydrationError> {
        for (i, root) in vnode.template.get().roots.iter().enumerate() {
            self.rehydrate_template_node(
                dom,
                vnode,
                root,
                ids,
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
                        mounted_id = Some(id);
                    }
                }
                if let Some(id) = mounted_id {
                    ids.push(id.0 as u32);
                }
                if !children.is_empty() {
                    for child in *children {
                        self.rehydrate_template_node(dom, vnode, child, ids, None)?;
                    }
                }
            }
            TemplateNode::Dynamic { id } | TemplateNode::DynamicText { id } => {
                self.rehydrate_dynamic_node(dom, &vnode.dynamic_nodes[*id], ids)?;
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
                self.rehydrate_scope(dom.get_scope(scope).ok_or(VNodeNotInitialized)?, dom, ids)?;
            }
            dioxus_core::DynamicNode::Fragment(fragment) => {
                for vnode in *fragment {
                    self.rehydrate_vnode(dom, vnode, ids)?;
                }
            }
        }
        Ok(())
    }
}
