use crate::innerlude::*;

use futures_channel::mpsc::UnboundedSender;
use fxhash::FxHashMap;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
    future::Future,
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
    // safety:
    //
    // pointers to scopes are *always* valid since they are bump allocated and never freed until this scope is also freed
    // this is just a bit of a hack to not need an Rc to the ScopeArena.
    // todo: replace this will ScopeId and provide a connection to scope arena directly
    pub(crate) parent_scope: Option<*mut Scope>,

    pub(crate) our_arena_idx: ScopeId,

    pub(crate) height: u32,

    pub(crate) subtree: Cell<u32>,

    pub(crate) is_subtree_root: Cell<bool>,

    pub(crate) generation: Cell<u32>,

    // The double-buffering situation that we will use
    pub(crate) frames: [BumpFrame; 2],

    pub(crate) old_root: RefCell<Option<NodeLink>>,
    pub(crate) new_root: RefCell<Option<NodeLink>>,

    pub(crate) caller: *const dyn Fn(&Scope) -> Element,

    /*
    we care about:
    - listeners (and how to call them when an event is triggered)
    - borrowed props (and how to drop them when the parent is dropped)
    - suspended nodes (and how to call their callback when their associated tasks are complete)
    */
    pub(crate) items: RefCell<SelfReferentialItems<'static>>,

    // State
    pub(crate) hooks: HookList,

    // todo: move this into a centralized place - is more memory efficient
    pub(crate) shared_contexts: RefCell<HashMap<TypeId, Rc<dyn Any>>>,

    pub(crate) sender: UnboundedSender<SchedulerMsg>,
}

pub struct SelfReferentialItems<'a> {
    pub(crate) listeners: Vec<&'a Listener<'a>>,
    pub(crate) borrowed_props: Vec<&'a VComponent<'a>>,
    pub(crate) suspended_nodes: FxHashMap<u64, &'a VSuspended<'a>>,
    pub(crate) tasks: Vec<BumpBox<'a, dyn Future<Output = ()>>>,
    pub(crate) pending_effects: Vec<BumpBox<'a, dyn FnMut()>>,
}

/// A component's unique identifier.
///
/// `ScopeId` is a `usize` that is unique across the entire VirtualDOM - but not unique across time. If a component is
/// unmounted, then the `ScopeId` will be reused for a new component.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ScopeId(pub usize);

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
    /// ```rust
    /// let mut dom = VirtualDom::new(|cx, props|cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    ///
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.subtree(), 0);
    /// ```
    pub fn subtree(&self) -> u32 {
        self.subtree.get()
    }

    /// Get the height of this Scope - IE the number of scopes above it.
    ///
    /// A Scope with a height of `0` is the root scope - there are no other scopes above it.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut dom = VirtualDom::new(|cx, props|cx.render(rsx!{ div {} }));
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
    /// let mut dom = VirtualDom::new(|cx, props|cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    ///
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.parent(), None);
    /// ```
    pub fn parent(&self) -> Option<ScopeId> {
        match self.parent_scope {
            Some(p) => Some(unsafe { &*p }.our_arena_idx),
            None => None,
        }
    }

    /// Get the ID of this Scope within this Dioxus VirtualDOM.
    ///
    /// This ID is not unique across Dioxus VirtualDOMs or across time. IDs will be reused when components are unmounted.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut dom = VirtualDom::new(|cx, props|cx.render(rsx!{ div {} }));
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
        // pub fn schedule_update(&self) -> Rc<dyn Fn() + 'static> {
        let chan = self.sender.clone();
        let id = self.scope_id();
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

    /// Get the [`ScopeId`] of a mounted component.
    ///
    /// `ScopeId` is not unique for the lifetime of the VirtualDom - a ScopeId will be reused if a component is unmounted.
    pub fn bump(&self) -> &Bump {
        &self.wip_frame().bump
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
    pub fn render<'src>(&'src self, lazy_nodes: Option<LazyNodes<'src, '_>>) -> Option<NodeLink> {
        let bump = &self.wip_frame().bump;
        let factory = NodeFactory { bump };
        let node = lazy_nodes.map(|f| f.call(factory))?;

        let idx = self
            .wip_frame()
            .add_node(unsafe { std::mem::transmute(node) });

        Some(NodeLink {
            gen_id: self.generation.get(),
            scope_id: self.our_arena_idx,
            link_idx: idx,
        })
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

        let mut items = self.items.borrow_mut();
        items.pending_effects.push(self_ref_fut);
        items.pending_effects.len() - 1
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

        let mut items = self.items.borrow_mut();
        items.tasks.push(self_ref_fut);
        items.tasks.len() - 1
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
    /// static App: FC<()> = |cx, props|{
    ///     cx.use_hook(|_| cx.provide_state(SharedState("world")), |_| {}, |_| {});
    ///     rsx!(cx, Child {})
    /// }
    ///
    /// static Child: FC<()> = |cx, props|{
    ///     let state = cx.consume_state::<SharedState>();
    ///     rsx!(cx, div { "hello {state.0}" })
    /// }
    /// ```
    pub fn provide_state<T>(&self, value: T)
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
    pub fn consume_state<T: 'static>(&self) -> Option<Rc<T>> {
        if let Some(shared) = self.shared_contexts.borrow().get(&TypeId::of::<T>()) {
            Some(shared.clone().downcast::<T>().unwrap())
        } else {
            let mut search_parent = self.parent_scope;

            while let Some(parent_ptr) = search_parent {
                let parent = unsafe { &*parent_ptr };
                if let Some(shared) = parent.shared_contexts.borrow().get(&TypeId::of::<T>()) {
                    return Some(shared.clone().downcast::<T>().unwrap());
                }
                search_parent = parent.parent_scope;
            }
            None
        }
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
    /// static App: FC<()> = |cx, props| {
    ///     todo!();
    ///     rsx!(cx, div { "Subtree {id}"})
    /// };
    /// ```
    pub fn create_subtree(&self) -> Option<u32> {
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
    /// static App: FC<()> = |cx, props| {
    ///     let id = cx.get_current_subtree();
    ///     rsx!(cx, div { "Subtree {id}"})
    /// };
    /// ```
    pub fn get_current_subtree(&self) -> u32 {
        self.subtree()
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
        if self.hooks.at_end() {
            self.hooks.push_hook(initializer(self.hooks.len()));
        }

        const HOOK_ERR_MSG: &str = r###"
Unable to retrieve the hook that was initialized at this index.
Consult the `rules of hooks` to understand how to use hooks properly.

You likely used the hook in a conditional. Hooks rely on consistent ordering between renders.
Functions prefixed with "use" should never be called conditionally.
"###;

        runner(self.hooks.next::<State>().expect(HOOK_ERR_MSG))
    }
}

// Important internal methods
impl Scope {
    /// The "work in progress frame" represents the frame that is currently being worked on.
    pub(crate) fn wip_frame(&self) -> &BumpFrame {
        match self.generation.get() & 1 == 0 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }
    pub(crate) fn fin_frame(&self) -> &BumpFrame {
        match self.generation.get() & 1 == 1 {
            true => &self.frames[0],
            false => &self.frames[1],
        }
    }

    pub unsafe fn reset_wip_frame(&self) {
        // todo: unsafecell or something
        let bump = self.wip_frame() as *const _ as *mut Bump;
        let g = &mut *bump;
        g.reset();
    }

    pub fn cycle_frame(&self) {
        self.generation.set(self.generation.get() + 1);
    }

    /// A safe wrapper around calling listeners
    pub(crate) fn call_listener(&self, event: UserEvent, element: ElementId) {
        let listners = &mut self.items.borrow_mut().listeners;

        let listener = listners.iter().find(|lis| {
            let search = lis;
            if search.event == event.name {
                let search_id = search.mounted_node.get();
                search_id.map(|f| f == element).unwrap_or(false)
            } else {
                false
            }
        });

        if let Some(listener) = listener {
            let mut cb = listener.callback.borrow_mut();
            if let Some(cb) = cb.as_mut() {
                (cb)(event.event);
            }
        } else {
            log::warn!("An event was triggered but there was no listener to handle it");
        }
    }

    // General strategy here is to load up the appropriate suspended task and then run it.
    // Suspended nodes cannot be called repeatedly.
    pub(crate) fn call_suspended_node<'a>(&'a mut self, task_id: u64) {
        let mut nodes = &mut self.items.get_mut().suspended_nodes;

        if let Some(suspended) = nodes.remove(&task_id) {
            let sus: &'a VSuspended<'static> = unsafe { &*suspended };
            let sus: &'a VSuspended<'a> = unsafe { std::mem::transmute(sus) };
            let mut boxed = sus.callback.borrow_mut().take().unwrap();
            let new_node: Element = boxed();
        }
    }

    // run the list of effects
    pub(crate) fn run_effects(&mut self) {
        for mut effect in self.items.get_mut().pending_effects.drain(..) {
            effect();
        }
    }

    pub(crate) fn new_subtree(&self) -> Option<u32> {
        todo!()
        // if self.is_subtree_root.get() {
        //     None
        // } else {
        //     let cur = self.shared.cur_subtree.get();
        //     self.shared.cur_subtree.set(cur + 1);
        //     Some(cur)
        // }
    }
}

pub struct BumpFrame {
    pub bump: Bump,
    pub nodes: RefCell<Vec<VNode<'static>>>,
}
impl BumpFrame {
    pub fn new() -> Self {
        let bump = Bump::new();

        let node = &*bump.alloc(VText {
            text: "asd",
            dom_id: Default::default(),
            is_static: false,
        });
        let nodes = RefCell::new(vec![VNode::Text(unsafe { std::mem::transmute(node) })]);
        Self { bump, nodes }
    }
    fn add_node(&self, node: VNode<'static>) -> usize {
        let mut nodes = self.nodes.borrow_mut();
        nodes.push(node);
        nodes.len() - 1
    }
}

/// An abstraction over internally stored data using a hook-based memory layout.
///
/// Hooks are allocated using Boxes and then our stored references are given out.
///
/// It's unsafe to "reset" the hooklist, but it is safe to add hooks into it.
///
/// Todo: this could use its very own bump arena, but that might be a tad overkill
#[derive(Default)]
pub(crate) struct HookList {
    arena: Bump,
    vals: RefCell<Vec<*mut dyn Any>>,
    idx: Cell<usize>,
}

impl HookList {
    pub(crate) fn next<T: 'static>(&self) -> Option<&mut T> {
        self.vals.borrow().get(self.idx.get()).and_then(|inn| {
            self.idx.set(self.idx.get() + 1);
            let raw_box = unsafe { &mut **inn };
            raw_box.downcast_mut::<T>()
        })
    }

    /// This resets the internal iterator count
    /// It's okay that we've given out each hook, but now we have the opportunity to give it out again
    /// Therefore, resetting is considered unsafe
    ///
    /// This should only be ran by Dioxus itself before "running scope".
    /// Dioxus knows how to descend through the tree to prevent mutable aliasing.
    pub(crate) unsafe fn reset(&self) {
        self.idx.set(0);
    }

    pub(crate) fn push_hook<T: 'static>(&self, new: T) {
        let val = self.arena.alloc(new);
        self.vals.borrow_mut().push(val)
    }

    pub(crate) fn len(&self) -> usize {
        self.vals.borrow().len()
    }

    pub(crate) fn cur_idx(&self) -> usize {
        self.idx.get()
    }

    pub(crate) fn at_end(&self) -> bool {
        self.cur_idx() >= self.len()
    }

    pub fn clear_hooks(&mut self) {
        self.vals.borrow_mut().drain(..).for_each(|state| {
            let as_mut = unsafe { &mut *state };
            let boxed = unsafe { bumpalo::boxed::Box::from_raw(as_mut) };
            drop(boxed);
        });
    }

    /// Get the ammount of memory a hooklist uses
    /// Used in heuristics
    pub fn get_hook_arena_size(&self) -> usize {
        self.arena.allocated_bytes()
    }
}

#[test]
fn sizeof() {
    dbg!(std::mem::size_of::<Scope>());
}
