use crate::innerlude::*;

use futures_util::FutureExt;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    future::Future,
    marker::PhantomData,
    ops::Deref,
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
///
/// ## Available Methods:
/// - render
/// - use_hook
/// - use_task
/// - use_suspense
/// - submit_task
/// - children
/// - use_effect
///
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

// We currently deref to props, but it might make more sense to deref to Scope?
// This allows for code that takes cx.xyz instead of cx.props.xyz
impl<'a, T> Deref for Context<'a, T> {
    type Target = &'a T;
    fn deref(&self) -> &Self::Target {
        &self.props
    }
}

impl<'src, P> Context<'src, P> {
    /// Access the children elements passed into the component
    ///
    /// This enables patterns where a component is passed children from its parent.
    ///
    /// ## Details
    ///
    /// Unlike React, Dioxus allows *only* lists of children to be passed from parent to child - not arbitrary functions
    /// or classes. If you want to generate nodes instead of accepting them as a list, consider declaring a closure
    /// on the props that takes Context.
    ///
    /// If a parent passes children into a component, the child will always re-render when the parent re-renders. In other
    /// words, a component cannot be automatically memoized if it borrows nodes from its parent, even if the component's
    /// props are valid for the static lifetime.
    ///
    /// ## Example
    ///
    /// ```rust
    /// const App: FC<()> = |cx| {
    ///     cx.render(rsx!{
    ///         CustomCard {
    ///             h1 {}
    ///             p {}
    ///         }
    ///     })
    /// }
    ///
    /// const CustomCard: FC<()> = |cx| {
    ///     cx.render(rsx!{
    ///         div {
    ///             h1 {"Title card"}
    ///             {cx.children()}
    ///         }
    ///     })
    /// }
    /// ```
    pub fn children(&self) -> &'src [VNode<'src>] {
        self.scope.child_nodes
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
        _cleanup: Cleanup,
    ) -> Output
    where
        State: 'static,
        Output: 'src,
        Init: FnOnce(usize) -> State,
        Run: FnOnce(&'src mut State) -> Output,
        Cleanup: FnOnce(State),
    {
        // If the idx is the same as the hook length, then we need to add the current hook
        if self.scope.hooks.at_end() {
            let new_state = initializer(self.scope.hooks.len());
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
            |_| false,
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
            move |_| UseContextHook {
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
                    log::debug!(
                        "Searching {:#?} for valid shared_context",
                        inner.our_arena_idx
                    );
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
                        match inner.parent_idx {
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

    /// `submit_task` will submit the future to be polled.
    ///
    /// This is useful when you have some async task that needs to be progressed.
    ///
    /// This method takes ownership over the task you've provided, and must return (). This means any work that needs to
    /// happen must occur within the future or scheduled for after the future completes (through schedule_update )
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
    pub fn submit_task(&self, task: FiberTask) -> TaskHandle {
        (self.scope.task_submitter)(task);
        TaskHandle { _p: PhantomData {} }
    }

    /// Awaits the given task, forcing the component to re-render when the value is ready.
    ///
    ///
    ///
    ///
    pub fn use_task<Out, Fut, Init>(&self, task_initializer: Init) -> (&TaskHandle, &Option<Out>)
    where
        Out: 'static,
        Fut: Future<Output = Out> + 'static,
        Init: FnOnce() -> Fut + 'src,
    {
        struct TaskHook<T> {
            task_dump: Rc<RefCell<Option<T>>>,
            value: Option<T>,
        }

        // whenever the task is complete, save it into th
        self.use_hook(
            move |hook_idx| {
                let task_fut = task_initializer();

                let task_dump = Rc::new(RefCell::new(None));

                let slot = task_dump.clone();
                let update = self.schedule_update();
                let originator = self.scope.our_arena_idx.clone();

                self.submit_task(Box::pin(task_fut.then(move |output| async move {
                    *slot.as_ref().borrow_mut() = Some(output);
                    update();
                    EventTrigger {
                        event: VirtualEvent::AsyncEvent { hook_idx },
                        originator,
                        priority: EventPriority::Low,
                        real_node_id: None,
                    }
                })));

                TaskHook {
                    task_dump,
                    value: None,
                }
            },
            |hook| {
                if let Some(val) = hook.task_dump.as_ref().borrow_mut().take() {
                    hook.value = Some(val);
                }
                (&TaskHandle { _p: PhantomData }, &hook.value)
            },
            |_| {},
        )
    }
}

pub(crate) struct SuspenseHook {
    pub value: Rc<RefCell<Option<Box<dyn Any>>>>,
    pub callback: SuspendedCallback,
    pub dom_node_id: Rc<Cell<RealDomNode>>,
}
type SuspendedCallback = Box<dyn for<'a> Fn(Context<'a, ()>) -> VNode<'a>>;

impl<'src, P> Context<'src, P> {
    /// Asynchronously render new nodes once the given future has completed.
    ///
    /// # Easda
    ///
    ///
    ///
    ///
    /// # Example
    ///
    ///
    pub fn use_suspense<Out, Fut, Cb>(
        self,
        task_initializer: impl FnOnce() -> Fut,
        user_callback: Cb,
    ) -> VNode<'src>
    where
        Fut: Future<Output = Out> + 'static,
        Out: 'static,
        Cb: for<'a> Fn(Context<'a, ()>, &Out) -> VNode<'a> + 'static,
    {
        self.use_hook(
            move |hook_idx| {
                let value = Rc::new(RefCell::new(None));

                let dom_node_id = Rc::new(RealDomNode::empty_cell());
                let domnode = dom_node_id.clone();

                let slot = value.clone();

                let callback: SuspendedCallback = Box::new(move |ctx: Context<()>| {
                    let v: std::cell::Ref<Option<Box<dyn Any>>> = slot.as_ref().borrow();
                    match v.as_ref() {
                        Some(a) => {
                            let v: &dyn Any = a.as_ref();
                            let real_val = v.downcast_ref::<Out>().unwrap();
                            user_callback(ctx, real_val)
                        }
                        None => {
                            //
                            VNode {
                                dom_id: RealDomNode::empty_cell(),
                                key: None,
                                kind: VNodeKind::Suspended {
                                    node: domnode.clone(),
                                },
                            }
                        }
                    }
                });

                let originator = self.scope.our_arena_idx.clone();
                let task_fut = task_initializer();
                let domnode = dom_node_id.clone();

                let slot = value.clone();
                self.submit_task(Box::pin(task_fut.then(move |output| async move {
                    // When the new value arrives, set the hooks internal slot
                    // Dioxus will call the user's callback to generate new nodes outside of the diffing system
                    *slot.borrow_mut() = Some(Box::new(output) as Box<dyn Any>);
                    EventTrigger {
                        event: VirtualEvent::SuspenseEvent { hook_idx, domnode },
                        originator,
                        priority: EventPriority::Low,
                        real_node_id: None,
                    }
                })));

                SuspenseHook {
                    value,
                    callback,
                    dom_node_id,
                }
            },
            move |hook| {
                let cx = Context {
                    scope: &self.scope,
                    props: &(),
                };
                (&hook.callback)(cx)
            },
            |_| {},
        )
    }
}

pub struct TaskHandle<'src> {
    _p: PhantomData<&'src ()>,
}
