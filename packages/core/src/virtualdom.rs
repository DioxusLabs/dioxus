use crate::any_props::VComponentProps;
use crate::arena::ElementPath;
use crate::component::{Component, IntoComponent};
use crate::diff::DirtyScope;
use crate::future_container::FutureQueue;
use crate::innerlude::SchedulerMsg;
use crate::mutations::Mutation;
use crate::nodes::{Template, TemplateId};
use crate::{
    arena::ElementId,
    scopes::{ScopeId, ScopeState},
};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use slab::Slab;
use std::collections::{BTreeSet, HashMap};

pub struct VirtualDom {
    pub(crate) templates: HashMap<TemplateId, Template<'static>>,
    pub(crate) elements: Slab<ElementPath>,
    pub(crate) scopes: Slab<ScopeState>,
    pub(crate) scope_stack: Vec<ScopeId>,
    pub(crate) element_stack: Vec<ElementId>,
    pub(crate) dirty_scopes: BTreeSet<DirtyScope>,
    pub(crate) pending_futures: FutureQueue,
    pub(crate) sender: UnboundedSender<SchedulerMsg>,
    pub(crate) receiver: UnboundedReceiver<SchedulerMsg>,
}

impl VirtualDom {
    pub fn new(app: Component<()>) -> Self {
        let (sender, receiver) = futures_channel::mpsc::unbounded();

        let mut res = Self {
            templates: Default::default(),
            scopes: Slab::default(),
            elements: Default::default(),
            scope_stack: Vec::new(),
            element_stack: vec![ElementId(0)],
            dirty_scopes: BTreeSet::new(),
            pending_futures: FutureQueue::new(sender.clone()),
            receiver,
            sender,
        };

        let props = Box::into_raw(Box::new(VComponentProps::new_empty(app)));

        let root = res.new_scope(props);

        assert_eq!(root, ScopeId(0));

        res
    }

    /// Render the virtualdom, without processing any suspense.
    pub fn rebuild<'a>(&'a mut self, mutations: &mut Vec<Mutation<'a>>) {
        // let root = self.scopes.get(0).unwrap();

        let root_node = unsafe { std::mem::transmute(self.run_scope(ScopeId(0))) };

        // let root_node = unsafe { std::mem::transmute(root.root_node()) };

        self.scope_stack.push(ScopeId(0));
        self.create(mutations, root_node);
        self.scope_stack.pop();
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

    /// Wait for futures internal to the virtualdom
    ///
    /// This is cancel safe, so if the future is dropped, you can push events into the virtualdom
    pub async fn wait_for_work(&mut self) {}

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
