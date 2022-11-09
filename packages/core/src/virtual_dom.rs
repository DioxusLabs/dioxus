use crate::any_props::VComponentProps;
use crate::arena::ElementPath;
use crate::component::Component;
use crate::diff::DirtyScope;
use crate::factory::RenderReturn;
use crate::innerlude::{Renderer, Scheduler, SchedulerMsg};
use crate::mutations::Mutation;
use crate::nodes::{Template, TemplateId};

use crate::{
    arena::ElementId,
    scopes::{ScopeId, ScopeState},
};
use crate::{scheduler, Element, Scope};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::Future;
use scheduler::{SuspenseBoundary, SuspenseContext, SuspenseId};
use slab::Slab;
use std::collections::{BTreeSet, HashMap};

pub struct VirtualDom {
    pub(crate) templates: HashMap<TemplateId, Template<'static>>,
    pub(crate) elements: Slab<ElementPath>,
    pub(crate) scopes: Slab<ScopeState>,
    pub(crate) element_stack: Vec<ElementId>,
    pub(crate) dirty_scopes: BTreeSet<DirtyScope>,
    pub(crate) scheduler: Scheduler,

    // While diffing we need some sort of way of breaking off a stream of suspended mutations.
    pub(crate) scope_stack: Vec<ScopeId>,
    pub(crate) waiting_on: Vec<SuspenseId>,
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
            waiting_on: Vec::new(),
            scheduler,
        };

        let props = Box::into_raw(Box::new(VComponentProps::new_empty(app)));
        let props: *mut VComponentProps<(), ()> = unsafe { std::mem::transmute(props) };

        let root = res.new_scope(props);

        // the root component is always a suspense boundary for any async children
        res.scopes[root.0].provide_context(SuspenseBoundary::new(root));
        assert_eq!(root, ScopeId(0));

        res
    }

    /// Render the virtualdom, without processing any suspense.
    ///
    /// This does register futures with wakers, but does not process any of them.
    pub fn rebuild<'a>(&'a mut self) -> Renderer<'a> {
        let mut mutations = Renderer::new(0);
        let root_node: &RenderReturn = self.run_scope(ScopeId(0));
        let root_node: &RenderReturn = unsafe { std::mem::transmute(root_node) };

        let mut created = 0;
        match root_node {
            RenderReturn::Sync(Some(node)) => {
                self.scope_stack.push(ScopeId(0));
                created = self.create(&mut mutations, node);
                self.scope_stack.pop();
            }
            RenderReturn::Sync(None) => {
                //
            }
            RenderReturn::Async(_) => unreachable!("Root scope cannot be an async component"),
        }

        mutations.push(Mutation::AppendChildren { m: created });

        mutations
    }

    /// Render what you can given the timeline and then move on
    ///
    /// It's generally a good idea to put some sort of limit on the suspense process in case a future is having issues.
    pub async fn render_with_deadline(
        &mut self,
        deadline: impl Future<Output = ()>,
    ) -> Vec<Mutation> {
        todo!()
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
