use crate::hooklist::HookList;
use crate::{arena::SharedArena, innerlude::*};
use appendlist::AppendList;
use bumpalo::Bump;
use slotmap::DefaultKey;
use slotmap::SlotMap;
use std::marker::PhantomData;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    future::Future,
    ops::Deref,
    pin::Pin,
    rc::{Rc, Weak},
};

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
/// fn example(cx: Context<Props>) -> VNode {
///     html! {
///         <div> "Hello, {cx.name}" </div>
///     }
/// }
/// ```
// todo: force lifetime of source into T as a valid lifetime too
// it's definitely possible, just needs some more messing around
pub struct Context<'src, T> {
    pub props: &'src T,
    pub scope: &'src Scope,
    pub tasks: &'src RefCell<Vec<&'src mut PinnedTask>>,
}
pub type PinnedTask = Pin<Box<dyn Future<Output = ()>>>;

impl<'src, T> Copy for Context<'src, T> {}
impl<'src, T> Clone for Context<'src, T> {
    fn clone(&self) -> Self {
        Self {
            props: self.props,
            scope: self.scope,
            tasks: self.tasks,
        }
    }
}

impl<'a, T> Deref for Context<'a, T> {
    type Target = &'a T;

    fn deref(&self) -> &Self::Target {
        &self.props
    }
}

impl<'src, P> Context<'src, P> {
    /// Access the children elements passed into the component
    pub fn children(&self) -> &'src [VNode<'src>] {
        // We're re-casting the nodes back out
        // They don't really have a static lifetime
        unsafe {
            let scope = self.scope;
            let nodes = scope.child_nodes;
            nodes
        }
    }

    /// Create a subscription that schedules a future render for the reference component
    pub fn schedule_update(&self) -> Rc<dyn Fn() + 'static> {
        self.scope.event_channel.clone()
    }

    pub fn schedule_effect(&self) -> Rc<dyn Fn() + 'static> {
        todo!()
    }

    pub fn schedule_layout_effect(&self) {
        todo!()
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
    pub fn render<F: FnOnce(NodeFactory<'src>) -> VNode<'src>>(
        self,
        lazy_nodes: LazyNodes<'src, F>,
    ) -> VNode<'src> {
        let scope_ref = self.scope;
        let listener_id = &scope_ref.listener_idx;
        lazy_nodes.into_vnode(NodeFactory {
            scope_ref,
            listener_id,
        })
    }

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
    pub fn use_hook<State, Output, Init, Run, Cleanup>(
        self,
        initializer: Init,
        runner: Run,
        cleanup: Cleanup,
    ) -> Output
    where
        State: 'static,
        Output: 'src,
        Init: FnOnce() -> State,
        Run: FnOnce(&'src mut State) -> Output,
        Cleanup: FnOnce(State),
    {
        // If the idx is the same as the hook length, then we need to add the current hook
        if self.scope.hooks.at_end() {
            let new_state = initializer();
            self.scope.hooks.push(new_state);
        }

        const ERR_MSG: &str = r###"
Unable to retrive the hook that was initialized in this index.
Consult the `rules of hooks` to understand how to use hooks properly.

You likely used the hook in a conditional. Hooks rely on consistent ordering between renders.
Any function prefixed with "use" should not be called conditionally.
"###;

        runner(self.scope.hooks.next::<State>().expect(ERR_MSG))
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
    pub fn use_create_context<T: 'static>(&self, init: impl Fn() -> T) {
        let mut cxs = self.scope.shared_contexts.borrow_mut();
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

        match (is_initialized, cxs.contains_key(&ty)) {
            // Do nothing, already initialized and already exists
            (true, true) => {}

            // Needs to be initialized
            (false, false) => {
                log::debug!("Initializing context...");
                cxs.insert(ty, Rc::new(init()));
            }

            _ => debug_assert!(false, "Cannot initialize two contexts of the same type"),
        }
    }

    /// There are hooks going on here!
    pub fn use_context<T: 'static>(self) -> &'src Rc<T> {
        self.try_use_context().unwrap()
    }

    /// Uses a context, storing the cached value around
    pub fn try_use_context<T: 'static>(self) -> Result<&'src Rc<T>> {
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
                let mut scope = Some(self.scope);

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

                    if let Some(shared_cx) = shared_contexts.get(&ty) {
                        log::debug!("found matching cx");
                        let rc = shared_cx
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

    pub fn suspend<Output: 'src, Fut: FnOnce(SuspendedContext, Output) -> VNode<'src> + 'src>(
        &'src self,
        fut: &'src mut Pin<Box<dyn Future<Output = Output> + 'static>>,
        callback: Fut,
    ) -> VNode<'src> {
        use futures_util::FutureExt;
        match fut.now_or_never() {
            Some(out) => {
                let suspended_cx = SuspendedContext {};
                let nodes = callback(suspended_cx, out);
                return nodes;
            }
            None => {
                // we need to register this task
                VNode::Suspended {
                    real: Cell::new(RealDomNode::empty()),
                }
            }
        }
    }

    /// `submit_task` will submit the future to be polled.
    ///
    /// This is useful when you have some async task that needs to be progressed.
    ///
    /// ## Explanation
    /// Dioxus will step its internal event loop if the future returns if the future completes while waiting.
    ///
    /// Tasks can't return anything, but they can be controlled with the returned handle
    ///
    /// Tasks will only run until the component renders again. Because `submit_task` is valid for the &'src lifetime, it
    /// is considered "stable"
    ///
    ///
    ///
    pub fn submit_task(&self, task: &'src mut PinnedTask) -> TaskHandle {
        self.tasks.borrow_mut().push(task);

        TaskHandle { _p: PhantomData {} }
    }
}

pub struct TaskHandle<'src> {
    _p: PhantomData<&'src ()>,
}
#[derive(Clone)]
pub struct SuspendedContext {}
