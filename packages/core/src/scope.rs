use crate::innerlude::*;

use futures_channel::mpsc::UnboundedSender;
use smallvec::SmallVec;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
    future::Future,
    pin::Pin,
    rc::Rc,
};

use bumpalo::{boxed::Box as BumpBox, Bump};

/// Components in Dioxus use the "Context" object to interact with their lifecycle.
///
/// This lets components access props, schedule updates, integrate hooks, and expose shared state.
///
/// For the most part, the only method you should be using regularly is `render`.
///
/// ## Example
///
/// ```ignore
/// #[derive(Props)]
/// struct ExampleProps {
///     name: String
/// }
///
/// fn Example(cx: Context, props: &ExampleProps) -> Element {
///     cx.render(rsx!{ div {"Hello, {props.name}"} })
/// }
/// ```
pub type Context<'a> = &'a Scope;

/// A component's unique identifier.
///
/// `ScopeId` is a `usize` that is unique across the entire VirtualDOM and across time. ScopeIDs will never be reused
/// once a component has been unmounted.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ScopeId(pub usize);

/// Every component in Dioxus is represented by a `Scope`.
///
/// Scopes contain the state for hooks, the component's props, and other lifecycle information.
///
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components.
/// The actual contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
///
/// We expose the `Scope` type so downstream users can traverse the Dioxus VirtualDOM for whatever
/// use case they might have.
pub struct Scope {
    pub(crate) parent_scope: Option<*mut Scope>,

    pub(crate) container: ElementId,

    pub(crate) our_arena_idx: ScopeId,

    pub(crate) height: u32,

    pub(crate) subtree: Cell<u32>,

    pub(crate) is_subtree_root: Cell<bool>,

    pub(crate) generation: Cell<u32>,

    pub(crate) frames: [BumpFrame; 2],

    pub(crate) caller: *const dyn Fn(&Scope) -> Element,

    pub(crate) items: RefCell<SelfReferentialItems<'static>>,

    pub(crate) hook_arena: Bump,
    pub(crate) hook_vals: RefCell<SmallVec<[*mut dyn Any; 5]>>,
    pub(crate) hook_idx: Cell<usize>,

    pub(crate) shared_contexts: RefCell<HashMap<TypeId, Rc<dyn Any>>>,

    pub(crate) sender: UnboundedSender<SchedulerMsg>,
}

pub struct SelfReferentialItems<'a> {
    pub(crate) listeners: Vec<&'a Listener<'a>>,
    pub(crate) borrowed_props: Vec<&'a VComponent<'a>>,
    pub(crate) tasks: Vec<Pin<BumpBox<'a, dyn Future<Output = ()>>>>,
}

// Public methods exposed to libraries and components
impl Scope {
    /// Get the subtree ID that this scope belongs to.
    ///
    /// Each component has its own subtree ID - the root subtree has an ID of 0. This ID is used by the renderer to route
    /// the mutations to the correct window/portal/subtree.
    ///
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new(|cx, props|cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    ///
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.subtree(), 0);
    /// ```
    ///
    /// todo: enable
    pub(crate) fn _subtree(&self) -> u32 {
        self.subtree.get()
    }

    /// Create a new subtree with this scope as the root of the subtree.
    ///
    /// Each component has its own subtree ID - the root subtree has an ID of 0. This ID is used by the renderer to route
    /// the mutations to the correct window/portal/subtree.
    ///
    /// This method
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// fn App(cx: Context, props: &()) -> Element {
    ///     todo!();
    ///     rsx!(cx, div { "Subtree {id}"})
    /// };
    /// ```
    ///
    /// todo: enable subtree
    pub(crate) fn _create_subtree(&self) -> Option<u32> {
        if self.is_subtree_root.get() {
            None
        } else {
            todo!()
        }
    }

    /// Get the height of this Scope - IE the number of scopes above it.
    ///
    /// A Scope with a height of `0` is the root scope - there are no other scopes above it.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new(|cx, props| cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    ///
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.height(), 0);
    /// ```
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the Parent of this Scope within this Dioxus VirtualDOM.
    ///
    /// This ID is not unique across Dioxus VirtualDOMs or across time. IDs will be reused when components are unmounted.
    ///
    /// The base component will not have a parent, and will return `None`.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new(|cx, props| cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    ///
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.parent(), None);
    /// ```
    pub fn parent(&self) -> Option<ScopeId> {
        // safety: the pointer to our parent is *always* valid thanks to the bump arena
        self.parent_scope.map(|p| unsafe { &*p }.our_arena_idx)
    }

    /// Get the ID of this Scope within this Dioxus VirtualDOM.
    ///
    /// This ID is not unique across Dioxus VirtualDOMs or across time. IDs will be reused when components are unmounted.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new(|cx, props| cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.scope_id(), 0);
    /// ```
    pub fn scope_id(&self) -> ScopeId {
        self.our_arena_idx
    }

    /// Create a subscription that schedules a future render for the reference component
    ///
    /// ## Notice: you should prefer using prepare_update and get_scope_id
    pub fn schedule_update(&self) -> Rc<dyn Fn() + 'static> {
        let (chan, id) = (self.sender.clone(), self.scope_id());
        Rc::new(move || {
            let _ = chan.unbounded_send(SchedulerMsg::Immediate(id));
        })
    }

    /// Schedule an update for any component given its ScopeId.
    ///
    /// A component's ScopeId can be obtained from `use_hook` or the [`Context::scope_id`] method.
    ///
    /// This method should be used when you want to schedule an update for a component
    pub fn schedule_update_any(&self) -> Rc<dyn Fn(ScopeId)> {
        let chan = self.sender.clone();
        Rc::new(move |id| {
            let _ = chan.unbounded_send(SchedulerMsg::Immediate(id));
        })
    }

    /// Get the [`ScopeId`] of a mounted component.
    ///
    /// `ScopeId` is not unique for the lifetime of the VirtualDom - a ScopeId will be reused if a component is unmounted.
    pub fn needs_update(&self) {
        self.needs_update_any(self.scope_id())
    }

    /// Get the [`ScopeId`] of a mounted component.
    ///
    /// `ScopeId` is not unique for the lifetime of the VirtualDom - a ScopeId will be reused if a component is unmounted.
    pub fn needs_update_any(&self, id: ScopeId) {
        let _ = self.sender.unbounded_send(SchedulerMsg::Immediate(id));
    }

    /// Get the Root Node of this scope
    pub fn root_node(&self) -> &VNode {
        todo!("Portals have changed how we address nodes. Still fixing this, sorry.");
        // let node = *self.wip_frame().nodes.borrow().get(0).unwrap();
        // unsafe { std::mem::transmute(&*node) }
    }

    /// This method enables the ability to expose state to children further down the VirtualDOM Tree.
    ///
    /// This is a "fundamental" operation and should only be called during initialization of a hook.
    ///
    /// For a hook that provides the same functionality, use `use_provide_state` and `use_consume_state` instead.
    ///
    /// When the component is dropped, so is the context. Be aware of this behavior when consuming
    /// the context via Rc/Weak.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// struct SharedState(&'static str);
    ///
    /// static App: FC<()> = |cx, props|{
    ///     cx.use_hook(|_| cx.provide_state(SharedState("world")), |_| {}, |_| {});
    ///     rsx!(cx, Child {})
    /// }
    ///
    /// static Child: FC<()> = |cx, props| {
    ///     let state = cx.consume_state::<SharedState>();
    ///     rsx!(cx, div { "hello {state.0}" })
    /// }
    /// ```
    pub fn provide_state<T: 'static>(&self, value: T) {
        self.shared_contexts
            .borrow_mut()
            .insert(TypeId::of::<T>(), Rc::new(value))
            .map(|f| f.downcast::<T>().ok())
            .flatten();
    }

    /// Try to retrieve a SharedState with type T from the any parent Scope.
    pub fn consume_state<T: 'static>(&self) -> Option<Rc<T>> {
        if let Some(shared) = self.shared_contexts.borrow().get(&TypeId::of::<T>()) {
            Some(shared.clone().downcast::<T>().unwrap())
        } else {
            let mut search_parent = self.parent_scope;

            while let Some(parent_ptr) = search_parent {
                // safety: all parent pointers are valid thanks to the bump arena
                let parent = unsafe { &*parent_ptr };
                if let Some(shared) = parent.shared_contexts.borrow().get(&TypeId::of::<T>()) {
                    return Some(shared.clone().downcast::<T>().unwrap());
                }
                search_parent = parent.parent_scope;
            }
            None
        }
    }

    /// Pushes the future onto the poll queue to be polled after the component renders.
    ///
    /// The future is forcibly dropped if the component is not ready by the next render
    pub fn push_task<'src, F>(&'src self, fut: impl FnOnce() -> F + 'src) -> usize
    where
        F: Future<Output = ()>,
        F::Output: 'src,
        F: 'src,
    {
        self.sender
            .unbounded_send(SchedulerMsg::NewTask(self.our_arena_idx))
            .unwrap();

        // wrap it in a type that will actually drop the contents
        //
        // Safety: we just made the pointer above and will promise not to alias it!
        // The main reason we do this through from_raw is because Bumpalo's box does
        // not support unsized coercion
        let fut: &mut dyn Future<Output = ()> = self.bump().alloc(fut());
        let boxed_fut: BumpBox<dyn Future<Output = ()>> = unsafe { BumpBox::from_raw(fut) };
        let pinned_fut: Pin<BumpBox<_>> = boxed_fut.into();

        // erase the 'src lifetime for self-referential storage
        let self_ref_fut = unsafe { std::mem::transmute(pinned_fut) };

        // Push the future into the tasks
        let mut items = self.items.borrow_mut();
        items.tasks.push(self_ref_fut);
        items.tasks.len() - 1
    }

    /// Take a lazy VNode structure and actually build it with the context of the VDom's efficient VNode allocator.
    ///
    /// This function consumes the context and absorb the lifetime, so these VNodes *must* be returned.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// fn Component(cx: Scope, props: &Props) -> Element {
    ///     // Lazy assemble the VNode tree
    ///     let lazy_nodes = rsx!("hello world");
    ///
    ///     // Actually build the tree and allocate it
    ///     cx.render(lazy_tree)
    /// }
    ///```
    pub fn render<'src>(&'src self, rsx: Option<LazyNodes<'src, '_>>) -> Option<VPortal> {
        let bump = &self.wip_frame().bump;

        let owned_node: VNode<'src> = rsx.map(|f| f.call(NodeFactory { bump }))?;
        let alloced_vnode: &'src mut VNode<'src> = bump.alloc(owned_node);
        let node_ptr: *mut VNode<'src> = alloced_vnode as *mut _;

        let node: *mut VNode<'static> = unsafe { std::mem::transmute(node_ptr) };

        Some(VPortal {
            scope_id: Cell::new(Some(self.our_arena_idx)),
            link_idx: Cell::new(0),
            node,
        })
    }

    /// Store a value between renders
    ///
    /// This is *the* foundational hook for all other hooks.
    ///
    /// - Initializer: closure used to create the initial hook state
    /// - Runner: closure used to output a value every time the hook is used
    ///
    /// To "cleanup" the hook, implement `Drop` on the stored hook value. Whenever the component is dropped, the hook
    /// will be dropped as well.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // use_ref is the simplest way of storing a value between renders
    /// fn use_ref<T: 'static>(initial_value: impl FnOnce() -> T) -> &RefCell<T> {
    ///     use_hook(
    ///         || Rc::new(RefCell::new(initial_value())),
    ///         |state| state,
    ///     )
    /// }
    /// ```
    pub fn use_hook<'src, State: 'static, Output: 'src>(
        &'src self,
        initializer: impl FnOnce(usize) -> State,
        runner: impl FnOnce(&'src mut State) -> Output,
    ) -> Output {
        let mut vals = self.hook_vals.borrow_mut();

        let hook_len = vals.len();
        let cur_idx = self.hook_idx.get();

        if cur_idx >= hook_len {
            vals.push(self.hook_arena.alloc(initializer(hook_len)));
        }

        let state = vals
            .get(cur_idx)
            .and_then(|inn| {
                self.hook_idx.set(cur_idx + 1);
                let raw_box = unsafe { &mut **inn };
                raw_box.downcast_mut::<State>()
            })
            .expect(
                r###"
                Unable to retrieve the hook that was initialized at this index.
                Consult the `rules of hooks` to understand how to use hooks properly.

                You likely used the hook in a conditional. Hooks rely on consistent ordering between renders.
                Functions prefixed with "use" should never be called conditionally.
                "###,
            );

        runner(state)
    }

    /// The "work in progress frame" represents the frame that is currently being worked on.
    pub(crate) fn wip_frame(&self) -> &BumpFrame {
        match self.generation.get() & 1 == 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }

    /// Mutable access to the "work in progress frame" - used to clear it
    pub(crate) fn wip_frame_mut(&mut self) -> &mut BumpFrame {
        match self.generation.get() & 1 == 0 {
            true => &mut self.frames[0],
            false => &mut self.frames[1],
        }
    }

    /// Access to the frame where finalized nodes existed
    pub(crate) fn fin_frame(&self) -> &BumpFrame {
        match self.generation.get() & 1 == 1 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }

    /// Reset this component's frame
    ///
    /// # Safety:
    ///
    /// This method breaks every reference of VNodes in the current frame.
    ///
    /// Calling reset itself is not usually a big deal, but we consider it important
    /// due to the complex safety guarantees we need to uphold.
    pub(crate) unsafe fn reset_wip_frame(&mut self) {
        self.wip_frame_mut().bump.reset();
    }

    /// Cycle to the next generation
    pub(crate) fn cycle_frame(&self) {
        self.generation.set(self.generation.get() + 1);
    }

    /// Get the [`Bump`] of the WIP frame.
    pub(crate) fn bump(&self) -> &Bump {
        &self.wip_frame().bump
    }
}

pub(crate) struct BumpFrame {
    pub bump: Bump,
    pub nodes: RefCell<Vec<*const VNode<'static>>>,
}
impl BumpFrame {
    pub(crate) fn new(capacity: usize) -> Self {
        let bump = Bump::with_capacity(capacity);

        let node = &*bump.alloc(VText {
            text: "asd",
            dom_id: Default::default(),
            is_static: false,
        });
        let node = bump.alloc(VNode::Text(unsafe { std::mem::transmute(node) }));
        let nodes = RefCell::new(vec![node as *const _]);
        Self { bump, nodes }
    }

    pub(crate) fn assign_nodelink(&self, node: &VPortal) {
        let mut nodes = self.nodes.borrow_mut();

        let len = nodes.len();
        nodes.push(node.node);

        node.link_idx.set(len);
    }
}

#[test]
fn sizeof() {
    dbg!(std::mem::size_of::<Scope>());
}
