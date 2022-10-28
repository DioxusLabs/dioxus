use crate::VirtualDom;

use crate::any_props::VComponentProps;
use crate::arena::ElementArena;
use crate::component::Component;
use crate::mutations::Mutation;
use crate::nodes::{DynamicNode, Template, TemplateId};
use crate::scopes::Scope;
use crate::{
    any_props::AnyProps,
    arena::ElementId,
    bump_frame::BumpFrame,
    nodes::VTemplate,
    scopes::{ComponentPtr, ScopeId, ScopeState},
};
use slab::Slab;

impl VirtualDom {}
