use crate::any_props::VComponentProps;
use crate::arena::ElementPath;
use crate::component::Component;
use crate::diff::DirtyScope;
use crate::factory::RenderReturn;
use crate::innerlude::{Scheduler, SchedulerMsg};
use crate::mutations::Mutation;
use crate::nodes::{Template, TemplateId};

use crate::{
    arena::ElementId,
    scopes::{ScopeId, ScopeState},
};
use crate::{scheduler, Element, Scope};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use scheduler::{SuspenseBoundary, SuspenseContext};
use slab::Slab;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

pub struct VirtualDom {
    pub(crate) templates: HashMap<TemplateId, Template<'static>>,
    pub(crate) elements: Slab<ElementPath>,
    pub(crate) scopes: Slab<ScopeState>,
    pub(crate) scope_stack: Vec<ScopeId>,
    pub(crate) element_stack: Vec<ElementId>,
    pub(crate) dirty_scopes: BTreeSet<DirtyScope>,
    pub(crate) scheduler: Scheduler,
}

impl VirtualDom {
    pub fn new(app: fn(Scope) -> Element) -> Self {
        let scheduler = Scheduler::new();

        let mut res = Self {
            templates: Default::default(),
            scopes: Slab::default(),
            elements: Default::default(),
            scope_stack: Vec::new(),
            element_stack: vec![ElementId(0)],
            dirty_scopes: BTreeSet::new(),
            scheduler,
        };

        let props = Box::into_raw(Box::new(VComponentProps::new_empty(app)));
        let props: *mut VComponentProps<(), ()> = unsafe { std::mem::transmute(props) };

        let root = res.new_scope(props);

        // the root component is always a suspense boundary for any async children
        res.scopes[root.0].provide_context(Rc::new(RefCell::new(SuspenseBoundary::new(root))));

        assert_eq!(root, ScopeId(0));

        res
    }

    /// Render the virtualdom, without processing any suspense.
    pub fn rebuild<'a>(&'a mut self, mutations: &mut Vec<Mutation<'a>>) {
        let root_node: &RenderReturn = self.run_scope(ScopeId(0));
        let root_node: &RenderReturn = unsafe { std::mem::transmute(root_node) };
        match root_node {
            RenderReturn::Sync(Some(node)) => {
                self.scope_stack.push(ScopeId(0));
                self.create(mutations, node);
                self.scope_stack.pop();
            }
            RenderReturn::Sync(None) => {
                //
            }
            RenderReturn::Async(_) => unreachable!(),
        }
    }

    /// Render what you can given the timeline and then move on
    pub async fn render_with_deadline<'a>(
        &'a mut self,
        future: impl std::future::Future<Output = ()>,
        mutations: &mut Vec<Mutation<'a>>,
    ) {
        todo!()
    }

    // Whenever the future is canceled, the VirtualDom will be
    pub async fn render<'a>(&'a mut self, mutations: &mut Vec<Mutation<'a>>) {
        //
    }

    pub fn get_scope(&self, id: ScopeId) -> Option<&ScopeState> {
        self.scopes.get(id.0)
    }

    pub fn base_scope(&self) -> &ScopeState {
        self.scopes.get(0).unwrap()
    }
}

impl Drop for VirtualDom {
    fn drop(&mut self) {
        // self.drop_scope(ScopeId(0));
    }
}
