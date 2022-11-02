use crate::virtualdom::VirtualDom;

use crate::any_props::VComponentProps;

use crate::component::Component;
use crate::mutations::Mutation;
use crate::nodes::{DynamicNode, Template, TemplateId};
use crate::scopes::Scope;
use crate::{
    any_props::AnyProps,
    arena::ElementId,
    bump_frame::BumpFrame,
    nodes::VNode,
    scopes::{ScopeId, ScopeState},
};
use slab::Slab;

pub struct DirtyScope {
    height: usize,
    id: ScopeId,
}

impl VirtualDom {
    fn diff_scope<'a>(&'a mut self, mutations: &mut Vec<Mutation<'a>>, scope: ScopeId) {
        let scope_state = &mut self.scopes[scope.0];
    }
}
