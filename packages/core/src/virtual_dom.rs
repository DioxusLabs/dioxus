use crate::any_props::VComponentProps;
use crate::arena::ElementPath;
use crate::diff::DirtyScope;
use crate::factory::RenderReturn;
use crate::innerlude::{Mutations, Scheduler, SchedulerMsg};
use crate::mutations::Mutation;
use crate::nodes::{Template, TemplateId};
use crate::{
    arena::ElementId,
    scopes::{ScopeId, ScopeState},
};
use crate::{scheduler, Element, Scope};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use scheduler::{SuspenseBoundary, SuspenseId};
use slab::Slab;
use std::collections::{BTreeSet, HashMap};
use std::future::Future;
use std::rc::Rc;

/// A virtual node system that progresses user events and diffs UI trees.
///
/// ## Guide
///
/// Components are defined as simple functions that take [`Scope`] and return an [`Element`].
///
/// ```rust, ignore
/// #[derive(Props, PartialEq)]
/// struct AppProps {
///     title: String
/// }
///
/// fn App(cx: Scope<AppProps>) -> Element {
///     cx.render(rsx!(
///         div {"hello, {cx.props.title}"}
///     ))
/// }
/// ```
///
/// Components may be composed to make complex apps.
///
/// ```rust, ignore
/// fn App(cx: Scope<AppProps>) -> Element {
///     cx.render(rsx!(
///         NavBar { routes: ROUTES }
///         Title { "{cx.props.title}" }
///         Footer {}
///     ))
/// }
/// ```
///
/// To start an app, create a [`VirtualDom`] and call [`VirtualDom::rebuild`] to get the list of edits required to
/// draw the UI.
///
/// ```rust, ignore
/// let mut vdom = VirtualDom::new(App);
/// let edits = vdom.rebuild();
/// ```
///
/// To inject UserEvents into the VirtualDom, call [`VirtualDom::get_scheduler_channel`] to get access to the scheduler.
///
/// ```rust, ignore
/// let channel = vdom.get_scheduler_channel();
/// channel.send_unbounded(SchedulerMsg::UserEvent(UserEvent {
///     // ...
/// }))
/// ```
///
/// While waiting for UserEvents to occur, call [`VirtualDom::wait_for_work`] to poll any futures inside the VirtualDom.
///
/// ```rust, ignore
/// vdom.wait_for_work().await;
/// ```
///
/// Once work is ready, call [`VirtualDom::work_with_deadline`] to compute the differences between the previous and
/// current UI trees. This will return a [`Mutations`] object that contains Edits, Effects, and NodeRefs that need to be
/// handled by the renderer.
///
/// ```rust, ignore
/// let mutations = vdom.work_with_deadline(|| false);
/// for edit in mutations {
///     apply(edit);
/// }
/// ```
///
/// ## Building an event loop around Dioxus:
///
/// Putting everything together, you can build an event loop around Dioxus by using the methods outlined above.
///
/// ```rust, ignore
/// fn App(cx: Scope) -> Element {
///     cx.render(rsx!{
///         div { "Hello World" }
///     })
/// }
///
/// async fn main() {
///     let mut dom = VirtualDom::new(App);
///
///     let mut inital_edits = dom.rebuild();
///     apply_edits(inital_edits);
///
///     loop {
///         dom.wait_for_work().await;
///         let frame_timeout = TimeoutFuture::new(Duration::from_millis(16));
///         let deadline = || (&mut frame_timeout).now_or_never();
///         let edits = dom.run_with_deadline(deadline).await;
///         apply_edits(edits);
///     }
/// }
/// ```
pub struct VirtualDom {
    pub(crate) templates: HashMap<TemplateId, Template<'static>>,
    pub(crate) elements: Slab<ElementPath>,
    pub(crate) scopes: Slab<ScopeState>,
    pub(crate) element_stack: Vec<ElementId>,
    pub(crate) dirty_scopes: BTreeSet<DirtyScope>,
    pub(crate) scheduler: Rc<Scheduler>,

    // While diffing we need some sort of way of breaking off a stream of suspended mutations.
    pub(crate) scope_stack: Vec<ScopeId>,
    pub(crate) collected_leaves: Vec<SuspenseId>,

    // Whenever a suspense tree is finished, we push its boundary onto this stack.
    // When "render_with_deadline" is called, we pop the stack and return the mutations
    pub(crate) finished_fibers: Vec<ScopeId>,

    pub(crate) rx: futures_channel::mpsc::UnboundedReceiver<SchedulerMsg>,
}

impl VirtualDom {
    /// Create a new VirtualDom with a component that does not have special props.
    ///
    /// # Description
    ///
    /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
    ///
    /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
    /// to toss out the entire tree.
    ///
    ///
    /// # Example
    /// ```rust, ignore
    /// fn Example(cx: Scope) -> Element  {
    ///     cx.render(rsx!( div { "hello world" } ))
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDom is not progressed, you must either "run_with_deadline" or use "rebuild" to progress it.
    pub fn new(app: fn(Scope) -> Element) -> Self {
        Self::new_with_props(app, ())
    }

    /// Create a new VirtualDom with the given properties for the root component.
    ///
    /// # Description
    ///
    /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
    ///
    /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
    /// to toss out the entire tree.
    ///
    ///
    /// # Example
    /// ```rust, ignore
    /// #[derive(PartialEq, Props)]
    /// struct SomeProps {
    ///     name: &'static str
    /// }
    ///
    /// fn Example(cx: Scope<SomeProps>) -> Element  {
    ///     cx.render(rsx!{ div{ "hello {cx.props.name}" } })
    /// }
    ///
    /// let dom = VirtualDom::new(Example);
    /// ```
    ///
    /// Note: the VirtualDom is not progressed on creation. You must either "run_with_deadline" or use "rebuild" to progress it.
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new_with_props(Example, SomeProps { name: "jane" });
    /// let mutations = dom.rebuild();
    /// ```
    pub fn new_with_props<P>(root: fn(Scope<P>) -> Element, root_props: P) -> Self
    where
        P: 'static,
    {
        let channel = futures_channel::mpsc::unbounded();
        Self::new_with_props_and_scheduler(root, root_props, channel)
    }

    /// Launch the VirtualDom, but provide your own channel for receiving and sending messages into the scheduler
    ///
    /// This is useful when the VirtualDom must be driven from outside a thread and it doesn't make sense to wait for the
    /// VirtualDom to be created just to retrieve its channel receiver.
    ///
    /// ```rust, ignore
    /// let channel = futures_channel::mpsc::unbounded();
    /// let dom = VirtualDom::new_with_scheduler(Example, (), channel);
    /// ```
    pub fn new_with_props_and_scheduler<P: 'static>(
        root: fn(Scope<P>) -> Element,
        root_props: P,
        (tx, rx): (
            UnboundedSender<SchedulerMsg>,
            UnboundedReceiver<SchedulerMsg>,
        ),
    ) -> Self {
        let mut dom = Self {
            rx,
            scheduler: Scheduler::new(tx),
            templates: Default::default(),
            scopes: Slab::default(),
            elements: Default::default(),
            scope_stack: Vec::new(),
            element_stack: vec![ElementId(0)],
            dirty_scopes: BTreeSet::new(),
            collected_leaves: Vec::new(),
            finished_fibers: Vec::new(),
        };

        dom.new_scope(Box::into_raw(Box::new(VComponentProps::new(
            root,
            |_, _| unreachable!(),
            root_props,
        ))))
        // The root component is always a suspense boundary for any async children
        // This could be unexpected, so we might rethink this behavior
        .provide_context(SuspenseBoundary::new(ScopeId(0)));

        dom
    }

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom from scratch.
    ///
    /// The diff machine expects the RealDom's stack to be the root of the application.
    ///
    /// Tasks will not be polled with this method, nor will any events be processed from the event queue. Instead, the
    /// root component will be ran once and then diffed. All updates will flow out as mutations.
    ///
    /// All state stored in components will be completely wiped away.
    ///
    /// Any templates previously registered will remain.
    ///
    /// # Example
    /// ```rust, ignore
    /// static App: Component = |cx|  cx.render(rsx!{ "hello world" });
    ///
    /// let mut dom = VirtualDom::new();
    /// let edits = dom.rebuild();
    ///
    /// apply_edits(edits);
    /// ```
    pub fn rebuild<'a>(&'a mut self) -> Mutations<'a> {
        let mut mutations = Mutations::new(0);

        let root_node = unsafe { self.run_scope_extend(ScopeId(0)) };
        match root_node {
            RenderReturn::Sync(Some(node)) => {
                let m = self.create_scope(ScopeId(0), &mut mutations, node);
                mutations.push(Mutation::AppendChildren { m });
            }
            RenderReturn::Sync(None) => {}
            RenderReturn::Async(_) => unreachable!("Root scope cannot be an async component"),
        }

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
