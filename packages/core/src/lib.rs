use std::collections::HashMap;

use crate::{
    any_props::AnyProps,
    arena::ElementId,
    bump_frame::BumpFrame,
    nodes::VTemplate,
    scopes::{ComponentPtr, ScopeId, ScopeState},
};
use any_props::VComponentProps;
use arena::ElementArena;
use component::Component;
use mutations::Mutation;
use nodes::{DynamicNode, Template, TemplateId};
use scopes::Scope;
use slab::Slab;

mod any_props;
mod arena;
mod bump_frame;
mod component;
mod create;
mod diff;
mod element;
mod mutations;
mod nodes;
mod scope_arena;
mod scopes;

pub struct VirtualDom {
    templates: HashMap<TemplateId, Template>,
    elements: ElementArena,
    scopes: Slab<ScopeState>,
    scope_stack: Vec<ScopeId>,
}

impl VirtualDom {
    pub fn new(app: Component) -> Self {
        let mut res = Self {
            templates: Default::default(),
            scopes: Slab::default(),
            elements: ElementArena::default(),
            scope_stack: Vec::new(),
        };

        res.new_scope(
            app as _,
            None,
            ElementId(0),
            Box::new(VComponentProps::new_empty(app)),
        );

        res
    }

    fn root_scope(&self) -> &ScopeState {
        todo!()
    }

    /// Render the virtualdom, waiting for all suspended nodes to complete before moving on
    ///
    /// Forces a full render of the virtualdom from scratch.
    ///
    /// Use other methods to update the virtualdom incrementally.
    pub fn render_all<'a>(&'a mut self, mutations: &mut Vec<Mutation<'a>>) {
        let root = self.root_scope();

        let root_template = root.current_arena();

        let root_node: &'a VTemplate = unsafe { &*root_template.node.get() };
        let root_node: &'a VTemplate<'a> = unsafe { std::mem::transmute(root_node) };

        self.create(mutations, root_node);
    }
}
