use crate::innerlude::*;

use fxhash::FxHashMap;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
    future::Future,
    pin::Pin,
    rc::Rc,
};

use crate::{innerlude::*, lazynodes::LazyNodes};
use bumpalo::{boxed::Box as BumpBox, Bump};
use std::ops::Deref;

/// Components in Dioxus use the "Context" object to interact with their lifecycle.
///
/// This lets components access props, schedule updates, integrate hooks, and expose shared state.
///
/// Note: all of these methods are *imperative* - they do not act as hooks! They are meant to be used by hooks
/// to provide complex behavior. For instance, calling "add_shared_state" on every render is considered a leak. This method
/// exists for the `use_provide_state` hook to provide a shared state object.
///
/// For the most part, the only method you should be using regularly is `render`.
///
/// ## Example
///
/// ```ignore
/// #[derive(Properties)]
/// struct Props {
///     name: String
/// }
///
/// fn example(cx: Context<Props>) -> VNode {
///     html! {
///         <div> "Hello, {cx.name}" </div>
///     }
/// }
/// ```
pub type Context<'a> = &'a ScopeInner;

/// Every component in Dioxus is represented by a `Scope`.
///
/// Scopes contain the state for hooks, the component's props, and other lifecycle information.
///
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components.
/// The actual contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
///
/// We expose the `Scope` type so downstream users can traverse the Dioxus VirtualDOM for whatever
/// use case they might have.
pub struct ScopeInner {
    // Book-keeping about our spot in the arena
    pub(crate) parent_idx: Option<ScopeId>,
    pub(crate) our_arena_idx: ScopeId,
    pub(crate) height: u32,
    pub(crate) subtree: Cell<u32>,
    pub(crate) is_subtree_root: Cell<bool>,

    // Nodes
    pub(crate) frames: ActiveFrame,
    pub(crate) caller: BumpBox<'static, dyn for<'b> Fn(&'b ScopeInner) -> Element<'b>>,

    /*
    we care about:
    - listeners (and how to call them when an event is triggered)
    - borrowed props (and how to drop them when the parent is dropped)
    - suspended nodes (and how to call their callback when their associated tasks are complete)
    */
    pub(crate) listeners: RefCell<Vec<*const Listener<'static>>>,
    pub(crate) borrowed_props: RefCell<Vec<*const VComponent<'static>>>,
    pub(crate) suspended_nodes: RefCell<FxHashMap<u64, *const VSuspended<'static>>>,

    pub(crate) tasks: RefCell<Vec<BumpBox<'static, dyn Future<Output = ()>>>>,
    pub(crate) pending_effects: RefCell<Vec<BumpBox<'static, dyn FnMut()>>>,

    // State
    pub(crate) hooks: HookList,

    // todo: move this into a centralized place - is more memory efficient
    pub(crate) shared_contexts: RefCell<HashMap<TypeId, Rc<dyn Any>>>,

    // whenever set_state is called, we fire off a message to the scheduler
    // this closure _is_ the method called by schedule_update that marks this component as dirty
    pub(crate) memoized_updater: Rc<dyn Fn()>,

    pub(crate) shared: EventChannel,
}

/// Public interface for Scopes.
impl ScopeInner {
    /// Get the root VNode for this Scope.
    ///
    /// This VNode is the "entrypoint" VNode. If the component renders multiple nodes, then this VNode will be a fragment.
    ///
    /// # Example
    /// ```rust
    /// let mut dom = VirtualDom::new(|(cx, props)|cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    ///
    /// let base = dom.base_scope();
    ///
    /// if let VNode::VElement(node) = base.root_node() {
    ///     assert_eq!(node.tag_name, "div");
    /// }
    /// ```
    pub fn root_node(&self) -> &VNode {
        self.frames.fin_head()
    }

    /// Get the subtree ID that this scope belongs to.
    ///
    /// Each component has its own subtree ID - the root subtree has an ID of 0. This ID is used by the renderer to route
    /// the mutations to the correct window/portal/subtree.
    ///
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut dom = VirtualDom::new(|(cx, props)|cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    ///
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.subtree(), 0);
    /// ```
    pub fn subtree(&self) -> u32 {
        self.subtree.get()
    }

    pub(crate) fn new_subtree(&self) -> Option<u32> {
        if self.is_subtree_root.get() {
            None
        } else {
            let cur = self.shared.cur_subtree.get();
            self.shared.cur_subtree.set(cur + 1);
            Some(cur)
        }
    }

    /// Get the height of this Scope - IE the number of scopes above it.
    ///
    /// A Scope with a height of `0` is the root scope - there are no other scopes above it.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut dom = VirtualDom::new(|(cx, props)|cx.render(rsx!{ div {} }));
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
    /// ```rust
    /// let mut dom = VirtualDom::new(|(cx, props)|cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    ///
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.parent(), None);
    /// ```
    pub fn parent(&self) -> Option<ScopeId> {
        self.parent_idx
    }

    /// Get the ID of this Scope within this Dioxus VirtualDOM.
    ///
    /// This ID is not unique across Dioxus VirtualDOMs or across time. IDs will be reused when components are unmounted.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut dom = VirtualDom::new(|(cx, props)|cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.scope_id(), 0);
    /// ```
    pub fn scope_id(&self) -> ScopeId {
        self.our_arena_idx
    }
}

// The type of closure that wraps calling components
/// The type of task that gets sent to the task scheduler
/// Submitting a fiber task returns a handle to that task, which can be used to wake up suspended nodes
pub type FiberTask = Pin<Box<dyn Future<Output = ScopeId>>>;

/// Private interface for Scopes.
impl ScopeInner {
    // we are being created in the scope of an existing component (where the creator_node lifetime comes into play)
    // we are going to break this lifetime by force in order to save it on ourselves.
    // To make sure that the lifetime isn't truly broken, we receive a Weak RC so we can't keep it around after the parent dies.
    // This should never happen, but is a good check to keep around
    //
    // Scopes cannot be made anywhere else except for this file
    // Therefore, their lifetimes are connected exclusively to the virtual dom
    pub(crate) fn new(
        caller: BumpBox<dyn for<'b> Fn(&'b ScopeInner) -> Element<'b>>,
        our_arena_idx: ScopeId,
        parent_idx: Option<ScopeId>,
        height: u32,
        subtree: u32,
        shared: EventChannel,
    ) -> Self {
        let schedule_any_update = shared.schedule_any_immediate.clone();

        let memoized_updater = Rc::new(move || schedule_any_update(our_arena_idx));

        // wipe away the associated lifetime - we are going to manually manage the one-way lifetime graph
        let caller = unsafe { std::mem::transmute(caller) };

        Self {
            memoized_updater,
            shared,
            caller,
            parent_idx,
            our_arena_idx,
            height,
            subtree: Cell::new(subtree),
            is_subtree_root: Cell::new(false),
            tasks: Default::default(),
            frames: ActiveFrame::new(),
            hooks: Default::default(),

            pending_effects: Default::default(),
            suspended_nodes: Default::default(),
            shared_contexts: Default::default(),
            listeners: Default::default(),
            borrowed_props: Default::default(),
        }
    }

    pub(crate) fn update_scope_dependencies(
        &mut self,
        caller: &dyn for<'b> Fn(&'b ScopeInner) -> Element<'b>,
    ) {
        log::debug!("Updating scope dependencies {:?}", self.our_arena_idx);
        let caller = caller as *const _;
        self.caller = unsafe { std::mem::transmute(caller) };
    }

    /// This method cleans up any references to data held within our hook list. This prevents mutable aliasing from
    /// causing UB in our tree.
    ///
    /// This works by cleaning up our references from the bottom of the tree to the top. The directed graph of components
    /// essentially forms a dependency tree that we can traverse from the bottom to the top. As we traverse, we remove
    /// any possible references to the data in the hook list.
    ///
    /// References to hook data can only be stored in listeners and component props. During diffing, we make sure to log
    /// all listeners and borrowed props so we can clear them here.
    ///
    /// This also makes sure that drop order is consistent and predictable. All resources that rely on being dropped will
    /// be dropped.
    pub(crate) fn ensure_drop_safety(&mut self, pool: &ResourcePool) {
        // make sure we drop all borrowed props manually to guarantee that their drop implementation is called before we
        // run the hooks (which hold an &mut Reference)
        // right now, we don't drop
        self.borrowed_props
            .get_mut()
            .drain(..)
            .map(|li| unsafe { &*li })
            .for_each(|comp| {
                // First drop the component's undropped references
                let scope_id = comp
                    .associated_scope
                    .get()
                    .expect("VComponents should be associated with a valid Scope");

                if let Some(scope) = pool.get_scope_mut(scope_id) {
                    scope.ensure_drop_safety(pool);

                    let mut drop_props = comp.drop_props.borrow_mut().take().unwrap();
                    drop_props();
                }
            });

        // Now that all the references are gone, we can safely drop our own references in our listeners.
        self.listeners
            .get_mut()
            .drain(..)
            .map(|li| unsafe { &*li })
            .for_each(|listener| drop(listener.callback.borrow_mut().take()));
    }

    /// A safe wrapper around calling listeners
    pub(crate) fn call_listener(&mut self, event: UserEvent, element: ElementId) {
        let listners = self.listeners.borrow_mut();

        let raw_listener = listners.iter().find(|lis| {
            let search = unsafe { &***lis };
            if search.event == event.name {
                let search_id = search.mounted_node.get();
                search_id.map(|f| f == element).unwrap_or(false)
            } else {
                false
            }
        });

        if let Some(raw_listener) = raw_listener {
            let listener = unsafe { &**raw_listener };
            let mut cb = listener.callback.borrow_mut();
            if let Some(cb) = cb.as_mut() {
                (cb)(event.event);
            }
        } else {
            log::warn!("An event was triggered but there was no listener to handle it");
        }
    }

    /*
    General strategy here is to load up the appropriate suspended task and then run it.
    Suspended nodes cannot be called repeatedly.
    */
    pub(crate) fn call_suspended_node<'a>(&'a mut self, task_id: u64) {
        let mut nodes = self.suspended_nodes.borrow_mut();

        if let Some(suspended) = nodes.remove(&task_id) {
            let sus: &'a VSuspended<'static> = unsafe { &*suspended };
            let sus: &'a VSuspended<'a> = unsafe { std::mem::transmute(sus) };
            let mut boxed = sus.callback.borrow_mut().take().unwrap();
            let new_node: Element<'a> = boxed();
        }
    }

    // run the list of effects
    pub(crate) fn run_effects(&mut self, pool: &ResourcePool) {
        todo!()
        // let mut effects = self.frames.effects.borrow_mut();
        // let mut effects = effects.drain(..).collect::<Vec<_>>();

        // for effect in effects {
        //     let effect = unsafe { &*effect };
        //     let effect = effect.as_ref();

        //     let mut effect = effect.borrow_mut();
        //     let mut effect = effect.as_mut();

        //     effect.run(pool);
        // }
    }

    /// Render this component.
    ///
    /// Returns true if the scope completed successfully and false if running failed (IE a None error was propagated).
    pub(crate) fn run_scope<'sel>(&'sel mut self, pool: &ResourcePool) -> bool {
        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        // Remove all the outdated listeners
        self.ensure_drop_safety(pool);

        // Safety:
        // - We dropped the listeners, so no more &mut T can be used while these are held
        // - All children nodes that rely on &mut T are replaced with a new reference
        unsafe { self.hooks.reset() };

        // Safety:
        // - We've dropped all references to the wip bump frame
        unsafe { self.frames.reset_wip_frame() };

        // just forget about our suspended nodes while we're at it
        self.suspended_nodes.get_mut().clear();

        // guarantee that we haven't screwed up - there should be no latent references anywhere
        debug_assert!(self.listeners.borrow().is_empty());
        debug_assert!(self.suspended_nodes.borrow().is_empty());
        debug_assert!(self.borrowed_props.borrow().is_empty());

        log::debug!("Borrowed stuff is successfully cleared");

        // Cast the caller ptr from static to one with our own reference
        let render: &dyn for<'b> Fn(&'b ScopeInner) -> Element<'b> = unsafe { &*self.caller };

        // Todo: see if we can add stronger guarantees around internal bookkeeping and failed component renders.
        if let Some(builder) = render(self) {
            let new_head = builder.into_vnode(NodeFactory {
                bump: &self.frames.wip_frame().bump,
            });
            log::debug!("Render is successful");

            // the user's component succeeded. We can safely cycle to the next frame
            self.frames.wip_frame_mut().head_node = unsafe { std::mem::transmute(new_head) };
            self.frames.cycle_frame();

            true
        } else {
            false
        }
    }
    /// Create a subscription that schedules a future render for the reference component
    ///
    /// ## Notice: you should prefer using prepare_update and get_scope_id
    pub fn schedule_update(&self) -> Rc<dyn Fn() + 'static> {
        self.memoized_updater.clone()
    }

    /// Get the [`ScopeId`] of a mounted component.
    ///
    /// `ScopeId` is not unique for the lifetime of the VirtualDom - a ScopeId will be reused if a component is unmounted.
    pub fn needs_update(&self) {
        (self.memoized_updater)()
    }

    /// Get the [`ScopeId`] of a mounted component.
    ///
    /// `ScopeId` is not unique for the lifetime of the VirtualDom - a ScopeId will be reused if a component is unmounted.
    pub fn needs_update_any(&self, id: ScopeId) {
        (self.shared.schedule_any_immediate)(id)
    }

    /// Schedule an update for any component given its ScopeId.
    ///
    /// A component's ScopeId can be obtained from `use_hook` or the [`Context::scope_id`] method.
    ///
    /// This method should be used when you want to schedule an update for a component
    pub fn schedule_update_any(&self) -> Rc<dyn Fn(ScopeId)> {
        self.shared.schedule_any_immediate.clone()
    }

    /// Get the [`ScopeId`] of a mounted component.
    ///
    /// `ScopeId` is not unique for the lifetime of the VirtualDom - a ScopeId will be reused if a component is unmounted.
    pub fn bump(&self) -> &Bump {
        let bump = &self.frames.wip_frame().bump;
        bump
    }

    /// Take a lazy VNode structure and actually build it with the context of the VDom's efficient VNode allocator.
    ///
    /// This function consumes the context and absorb the lifetime, so these VNodes *must* be returned.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// fn Component(cx: Context<()>) -> VNode {
    ///     // Lazy assemble the VNode tree
    ///     let lazy_tree = html! {<div> "Hello World" </div>};
    ///
    ///     // Actually build the tree and allocate it
    ///     cx.render(lazy_tree)
    /// }
    ///```
    pub fn render<'src>(
        &'src self,
        lazy_nodes: Option<LazyNodes<'src, '_>>,
    ) -> Option<VNode<'src>> {
        let bump = &self.frames.wip_frame().bump;
        let factory = NodeFactory { bump };
        lazy_nodes.map(|f| f.call(factory))
    }

    /// Push an effect to be ran after the component has been successfully mounted to the dom
    /// Returns the effect's position in the stack
    pub fn push_effect<'src>(&'src self, effect: impl FnOnce() + 'src) -> usize {
        // this is some tricker to get around not being able to actually call fnonces
        let mut slot = Some(effect);
        let fut: &mut dyn FnMut() = self.bump().alloc(move || slot.take().unwrap()());

        // wrap it in a type that will actually drop the contents
        let boxed_fut = unsafe { BumpBox::from_raw(fut) };

        // erase the 'src lifetime for self-referential storage
        let self_ref_fut = unsafe { std::mem::transmute(boxed_fut) };

        self.pending_effects.borrow_mut().push(self_ref_fut);
        self.pending_effects.borrow().len() - 1
    }

    /// Pushes the future onto the poll queue to be polled
    /// The future is forcibly dropped if the component is not ready by the next render
    pub fn push_task<'src>(&'src self, fut: impl Future<Output = ()> + 'src) -> usize {
        // allocate the future
        let fut: &mut dyn Future<Output = ()> = self.bump().alloc(fut);

        // wrap it in a type that will actually drop the contents
        let boxed_fut: BumpBox<dyn Future<Output = ()>> = unsafe { BumpBox::from_raw(fut) };

        // erase the 'src lifetime for self-referential storage
        let self_ref_fut = unsafe { std::mem::transmute(boxed_fut) };

        self.tasks.borrow_mut().push(self_ref_fut);
        self.tasks.borrow().len() - 1
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
    /// ```
    /// struct SharedState(&'static str);
    ///
    /// static App: FC<()> = |(cx, props)|{
    ///     cx.use_hook(|_| cx.provide_state(SharedState("world")), |_| {}, |_| {});
    ///     rsx!(cx, Child {})
    /// }
    ///
    /// static Child: FC<()> = |(cx, props)|{
    ///     let state = cx.consume_state::<SharedState>();
    ///     rsx!(cx, div { "hello {state.0}" })
    /// }
    /// ```
    pub fn provide_state<T>(self, value: T)
    where
        T: 'static,
    {
        self.shared_contexts
            .borrow_mut()
            .insert(TypeId::of::<T>(), Rc::new(value))
            .map(|f| f.downcast::<T>().ok())
            .flatten();
    }

    /// Try to retrieve a SharedState with type T from the any parent Scope.
    pub fn consume_state<T: 'static>(self) -> Option<Rc<T>> {
        let getter = &self.shared.get_shared_context;
        let ty = TypeId::of::<T>();
        let idx = self.our_arena_idx;
        getter(idx, ty).map(|f| f.downcast().unwrap())
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
    /// ```rust
    /// static App: FC<()> = |(cx, props)| {
    ///     todo!();
    ///     rsx!(cx, div { "Subtree {id}"})
    /// };
    /// ```
    pub fn create_subtree(self) -> Option<u32> {
        self.new_subtree()
    }

    /// Get the subtree ID that this scope belongs to.
    ///
    /// Each component has its own subtree ID - the root subtree has an ID of 0. This ID is used by the renderer to route
    /// the mutations to the correct window/portal/subtree.
    ///
    /// # Example
    ///
    /// ```rust
    /// static App: FC<()> = |(cx, props)| {
    ///     let id = cx.get_current_subtree();
    ///     rsx!(cx, div { "Subtree {id}"})
    /// };
    /// ```
    pub fn get_current_subtree(self) -> u32 {
        self.subtree()
    }

    /// Store a value between renders
    ///
    /// This is *the* foundational hook for all other hooks.
    ///
    /// - Initializer: closure used to create the initial hook state
    /// - Runner: closure used to output a value every time the hook is used
    /// - Cleanup: closure used to teardown the hook once the dom is cleaned up
    ///
    ///
    /// # Example
    ///
    /// ```ignore
    /// // use_ref is the simplest way of storing a value between renders
    /// fn use_ref<T: 'static>(initial_value: impl FnOnce() -> T) -> &RefCell<T> {
    ///     use_hook(
    ///         || Rc::new(RefCell::new(initial_value())),
    ///         |state| state,
    ///         |_| {},
    ///     )
    /// }
    /// ```
    pub fn use_hook<'src, State, Output, Init, Run, Cleanup>(
        &'src self,
        initializer: Init,
        runner: Run,
        cleanup: Cleanup,
    ) -> Output
    where
        State: 'static,
        Output: 'src,
        Init: FnOnce(usize) -> State,
        Run: FnOnce(&'src mut State) -> Output,
        Cleanup: FnOnce(Box<State>) + 'static,
    {
        // If the idx is the same as the hook length, then we need to add the current hook
        if self.hooks.at_end() {
            self.hooks.push_hook(
                initializer(self.hooks.len()),
                Box::new(|raw| {
                    let s = raw.downcast::<State>().unwrap();
                    cleanup(s);
                }),
            );
        }

        runner(self.hooks.next::<State>().expect(HOOK_ERR_MSG))
    }
}

const HOOK_ERR_MSG: &str = r###"
Unable to retrieve the hook that was initialized at this index.
Consult the `rules of hooks` to understand how to use hooks properly.

You likely used the hook in a conditional. Hooks rely on consistent ordering between renders.
Functions prefixed with "use" should never be called conditionally.
"###;
