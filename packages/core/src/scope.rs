use crate::{arena::ScopeArena, innerlude::*};
use bumpalo::Bump;
use generational_arena::Arena;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    future::Future,
    ops::Deref,
    pin::Pin,
    rc::{Rc, Weak},
};

/// Every component in Dioxus is represented by a `Scope`.
///
/// Scopes contain the state for hooks, the component's props, and other lifecycle information.
///
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components.
/// The actual contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
pub struct Scope {
    // The parent's scope ID
    pub parent: Option<ScopeIdx>,

    // IDs of children that this scope has created
    // This enables us to drop the children and their children when this scope is destroyed
    pub(crate) descendents: RefCell<HashSet<ScopeIdx>>,

    pub(crate) child_nodes: &'static [VNode<'static>],

    // A reference to the list of components.
    // This lets us traverse the component list whenever we need to access our parent or children.
    pub(crate) arena_link: ScopeArena,

    pub shared_contexts: RefCell<HashMap<TypeId, Rc<dyn Any>>>,

    // Our own ID accessible from the component map
    pub arena_idx: ScopeIdx,

    pub height: u32,

    pub event_channel: Rc<dyn Fn() + 'static>,

    // pub event_queue: EventQueue,
    pub caller: Weak<OpaqueComponent<'static>>,

    pub hookidx: RefCell<usize>,

    // ==========================
    // slightly unsafe stuff
    // ==========================
    // an internal, highly efficient storage of vnodes
    pub frames: ActiveFrame,

    // These hooks are actually references into the hook arena
    // These two could be combined with "OwningRef" to remove unsafe usage
    // or we could dedicate a tiny bump arena just for them
    // could also use ourborous
    hooks: RefCell<Vec<Hook>>,

    // Unsafety:
    // - is self-refenrential and therefore needs to point into the bump
    // Stores references into the listeners attached to the vnodes
    // NEEDS TO BE PRIVATE
    pub(crate) listeners: RefCell<Vec<*const dyn Fn(VirtualEvent)>>,
}

// We need to pin the hook so it doesn't move as we initialize the list of hooks
type Hook = Pin<Box<dyn std::any::Any>>;
type EventChannel = Rc<dyn Fn()>;

impl Scope {
    // we are being created in the scope of an existing component (where the creator_node lifetime comes into play)
    // we are going to break this lifetime by force in order to save it on ourselves.
    // To make sure that the lifetime isn't truly broken, we receive a Weak RC so we can't keep it around after the parent dies.
    // This should never happen, but is a good check to keep around
    //
    // Scopes cannot be made anywhere else except for this file
    // Therefore, their lifetimes are connected exclusively to the virtual dom
    pub(crate) fn new<'creator_node>(
        caller: Weak<OpaqueComponent<'creator_node>>,
        arena_idx: ScopeIdx,
        parent: Option<ScopeIdx>,
        height: u32,
        event_channel: EventChannel,
        arena_link: ScopeArena,
        child_nodes: &'creator_node [VNode<'creator_node>],
    ) -> Self {
        log::debug!(
            "New scope created, height is {}, idx is {:?}",
            height,
            arena_idx
        );

        // The function to run this scope is actually located in the parent's bump arena.
        // Every time the parent is updated, that function is invalidated via double-buffering wiping the old frame.
        // If children try to run this invalid caller, it *will* result in UB.
        //
        // During the lifecycle progression process, this caller will need to be updated. Right now,
        // until formal safety abstractions are implemented, we will just use unsafe to "detach" the caller
        // lifetime from the bump arena, exposing ourselves to this potential for invalidation. Truthfully,
        // this is a bit of a hack, but will remain this way until we've figured out a cleaner solution.
        //
        // Not the best solution, so TODO on removing this in favor of a dedicated resource abstraction.
        let caller = unsafe {
            std::mem::transmute::<
                Weak<OpaqueComponent<'creator_node>>,
                Weak<OpaqueComponent<'static>>,
            >(caller)
        };

        Self {
            child_nodes: &[],
            caller,
            parent,
            arena_idx,
            height,
            event_channel,
            arena_link,
            frames: ActiveFrame::new(),
            hooks: Default::default(),
            shared_contexts: Default::default(),
            listeners: Default::default(),
            hookidx: Default::default(),
            descendents: Default::default(),
        }
    }

    pub fn update_caller<'creator_node>(&mut self, caller: Weak<OpaqueComponent<'creator_node>>) {
        let broken_caller = unsafe {
            std::mem::transmute::<
                Weak<OpaqueComponent<'creator_node>>,
                Weak<OpaqueComponent<'static>>,
            >(caller)
        };

        self.caller = broken_caller;
    }

    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    ///
    /// Props is ?Sized because we borrow the props and don't need to know the size. P (sized) is used as a marker (unsized)
    pub fn run_scope<'sel>(&'sel mut self) -> Result<()> {
        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        self.frames.next().bump.reset();

        // Remove all the outdated listeners
        //
        self.listeners
            .try_borrow_mut()
            .ok()
            .ok_or(Error::FatalInternal("Borrowing listener failed"))?
            .drain(..);

        *self.hookidx.borrow_mut() = 0;

        let caller = self
            .caller
            .upgrade()
            .ok_or(Error::FatalInternal("Failed to get caller"))?;

        // Cast the caller ptr from static to one with our own reference
        let c2: &OpaqueComponent<'static> = caller.as_ref();
        let c3: &OpaqueComponent<'_> = unsafe { std::mem::transmute(c2) };

        let unsafe_head = unsafe { self.own_vnodes(c3) };

        self.frames.cur_frame_mut().head_node = unsafe_head;

        Ok(())
    }

    // this is its own function so we can preciesly control how lifetimes flow
    unsafe fn own_vnodes<'a>(&'a self, f: &OpaqueComponent<'a>) -> VNode<'static> {
        let new_head: VNode<'a> = f(self);
        let out: VNode<'static> = std::mem::transmute(new_head);
        out
    }

    // A safe wrapper around calling listeners
    // calling listeners will invalidate the list of listeners
    // The listener list will be completely drained because the next frame will write over previous listeners
    pub fn call_listener(&mut self, trigger: EventTrigger) -> Result<()> {
        let EventTrigger {
            listener_id, event, ..
        } = trigger;
        //
        unsafe {
            // Convert the raw ptr into an actual object
            // This operation is assumed to be safe
            let listener_fn = self
                .listeners
                .try_borrow()
                .ok()
                .ok_or(Error::FatalInternal("Borrowing listener failed"))?
                .get(listener_id as usize)
                .ok_or(Error::FatalInternal("Event should exist if triggered"))?
                .as_ref()
                .ok_or(Error::FatalInternal("Raw event ptr is invalid"))?;

            // Run the callback with the user event
            listener_fn(event);
        }
        Ok(())
    }

    pub fn next_frame<'bump>(&'bump self) -> &'bump VNode<'bump> {
        self.frames.current_head_node()
    }

    pub fn old_frame<'bump>(&'bump self) -> &'bump VNode<'bump> {
        self.frames.prev_head_node()
    }

    pub fn cur_frame(&self) -> &BumpFrame {
        self.frames.cur_frame()
    }
}

/// Components in Dioxus use the "Context" object to interact with their lifecycle.
/// This lets components schedule updates, integrate hooks, and expose their context via the context api.
///
/// Properties passed down from the parent component are also directly accessible via the exposed "props" field.
///
/// ```ignore
/// #[derive(Properties)]
/// struct Props {
///     name: String
///
/// }
///
/// fn example(ctx: Context, props: &Props -> VNode {
///     html! {
///         <div> "Hello, {ctx.ctx.name}" </div>
///     }
/// }
/// ```
// todo: force lifetime of source into T as a valid lifetime too
// it's definitely possible, just needs some more messing around

pub struct Context<'src, T> {
    pub props: &'src T,
    pub scope: &'src Scope,
}

impl<'src, T> Copy for Context<'src, T> {}
impl<'src, T> Clone for Context<'src, T> {
    fn clone(&self) -> Self {
        Self {
            props: self.props,
            scope: self.scope,
        }
    }
}

impl<'a, T> Deref for Context<'a, T> {
    type Target = &'a T;

    fn deref(&self) -> &Self::Target {
        &self.props
    }
}

impl<'src, T> Scoped<'src> for Context<'src, T> {
    fn get_scope(&self) -> &'src Scope {
        self.scope
    }
}

pub trait Scoped<'src>: Sized {
    fn get_scope(&self) -> &'src Scope;

    /// Access the children elements passed into the component
    fn children(&self) -> &'src [VNode<'src>] {
        // We're re-casting the nodes back out
        // They don't really have a static lifetime
        unsafe {
            let scope = self.get_scope();
            let nodes: &'src [VNode<'static>] = scope.child_nodes;

            // cast the lifetime back correctly
            std::mem::transmute(nodes)
        }
    }

    /// Create a subscription that schedules a future render for the reference component
    fn schedule_update(&self) -> Rc<dyn Fn() + 'static> {
        self.get_scope().event_channel.clone()
    }

    /// Take a lazy VNode structure and actually build it with the context of the VDom's efficient VNode allocator.
    ///
    /// This function consumes the context and absorb the lifetime, so these VNodes *must* be returned.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// fn Component(ctx: Context<()>) -> VNode {
    ///     // Lazy assemble the VNode tree
    ///     let lazy_tree = html! {<div> "Hello World" </div>};
    ///     
    ///     // Actually build the tree and allocate it
    ///     ctx.render(lazy_tree)
    /// }
    ///```
    fn render<'a, F: for<'b> FnOnce(&'b NodeCtx<'src>) -> VNode<'src> + 'src + 'a>(
        self,
        lazy_nodes: LazyNodes<'src, F>,
    ) -> VNode<'src> {
        lazy_nodes.into_vnode(&NodeCtx {
            scope_ref: self.get_scope(),
            listener_id: 0.into(),
        })
    }

    // impl<'scope> Context<'scope> {
    /// Store a value between renders
    ///
    /// - Initializer: closure used to create the initial hook state
    /// - Runner: closure used to output a value every time the hook is used
    /// - Cleanup: closure used to teardown the hook once the dom is cleaned up
    ///
    /// ```ignore
    /// // use_ref is the simplest way of storing a value between renders
    /// pub fn use_ref<T: 'static>(initial_value: impl FnOnce() -> T + 'static) -> Rc<RefCell<T>> {
    ///     use_hook(
    ///         || Rc::new(RefCell::new(initial_value())),
    ///         |state| state.clone(),
    ///         |_| {},
    ///     )
    /// }
    /// ```
    fn use_hook<InternalHookState: 'static, Output: 'src>(
        &self,

        // The closure that builds the hook state
        initializer: impl FnOnce() -> InternalHookState,

        // The closure that takes the hookstate and returns some value
        runner: impl FnOnce(&'src mut InternalHookState) -> Output,

        // The closure that cleans up whatever mess is left when the component gets torn down
        // TODO: add this to the "clean up" group for when the component is dropped
        _cleanup: impl FnOnce(InternalHookState),
    ) -> Output {
        let scope = self.get_scope();

        let idx = *scope.hookidx.borrow();

        // Grab out the hook list
        let mut hooks = scope.hooks.borrow_mut();

        // If the idx is the same as the hook length, then we need to add the current hook
        if idx >= hooks.len() {
            let new_state = initializer();
            hooks.push(Box::pin(new_state));
        }

        *scope.hookidx.borrow_mut() += 1;

        let stable_ref = hooks
            .get_mut(idx)
            .expect("Should not fail, idx is validated")
            .as_mut();

        let pinned_state = unsafe { Pin::get_unchecked_mut(stable_ref) };

        let internal_state = pinned_state.downcast_mut::<InternalHookState>().expect(
            r###"
Unable to retrive the hook that was initialized in this index.
Consult the `rules of hooks` to understand how to use hooks properly.

You likely used the hook in a conditional. Hooks rely on consistent ordering between renders.
Any function prefixed with "use" should not be called conditionally.
            "###,
        );

        // We extend the lifetime of the internal state
        runner(unsafe { &mut *(internal_state as *mut _) })
    }

    /// This hook enables the ability to expose state to children further down the VirtualDOM Tree.
    ///
    /// This is a hook, so it may not be called conditionally!
    ///
    /// The init method is ran *only* on first use, otherwise it is ignored. However, it uses hooks (ie `use`)
    /// so don't put it in a conditional.
    ///
    /// When the component is dropped, so is the context. Be aware of this behavior when consuming
    /// the context via Rc/Weak.
    ///
    ///
    ///
    fn use_create_context<T: 'static>(&self, init: impl Fn() -> T) {
        let scope = self.get_scope();
        let mut ctxs = scope.shared_contexts.borrow_mut();
        let ty = TypeId::of::<T>();

        let is_initialized = self.use_hook(
            || false,
            |s| {
                let i = s.clone();
                *s = true;
                i
            },
            |_| {},
        );

        match (is_initialized, ctxs.contains_key(&ty)) {
            // Do nothing, already initialized and already exists
            (true, true) => {}

            // Needs to be initialized
            (false, false) => {
                log::debug!("Initializing context...");
                ctxs.insert(ty, Rc::new(init()));
            }

            _ => debug_assert!(false, "Cannot initialize two contexts of the same type"),
        }
    }

    /// There are hooks going on here!
    fn use_context<T: 'static>(&self) -> &'src Rc<T> {
        self.try_use_context().unwrap()
    }

    /// Uses a context, storing the cached value around
    fn try_use_context<T: 'static>(&self) -> Result<&'src Rc<T>> {
        struct UseContextHook<C> {
            par: Option<Rc<C>>,
            we: Option<Weak<C>>,
        }

        self.use_hook(
            move || UseContextHook {
                par: None as Option<Rc<T>>,
                we: None as Option<Weak<T>>,
            },
            move |hook| {
                let scope = self.get_scope();
                let mut scope = Some(scope);

                if let Some(we) = &hook.we {
                    if let Some(re) = we.upgrade() {
                        hook.par = Some(re);
                        return Ok(hook.par.as_ref().unwrap());
                    }
                }

                let ty = TypeId::of::<T>();
                while let Some(inner) = scope {
                    log::debug!("Searching {:#?} for valid shared_context", inner.arena_idx);
                    let shared_contexts = inner.shared_contexts.borrow();

                    if let Some(shared_ctx) = shared_contexts.get(&ty) {
                        log::debug!("found matching ctx");
                        let rc = shared_ctx
                            .clone()
                            .downcast::<T>()
                            .expect("Should not fail, already validated the type from the hashmap");

                        hook.we = Some(Rc::downgrade(&rc));
                        hook.par = Some(rc);
                        return Ok(hook.par.as_ref().unwrap());
                    } else {
                        match inner.parent {
                            Some(parent_id) => {
                                let parent = inner
                                    .arena_link
                                    .try_get(parent_id)
                                    .map_err(|_| Error::FatalInternal("Failed to find parent"))?;

                                scope = Some(parent);
                            }
                            None => return Err(Error::MissingSharedContext),
                        }
                    }
                }

                Err(Error::MissingSharedContext)
            },
            |_| {},
        )
    }
}
