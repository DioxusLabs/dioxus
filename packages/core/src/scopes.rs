use crate::{
    any_props::{BoxedAnyProps, VProps},
    innerlude::{DynamicNode, EventHandler, VComponent, VText},
    nodes::{IntoAttributeValue, IntoDynNode, RenderReturn},
    runtime::Runtime,
    scope_context::ScopeContext,
    AnyValue, Attribute, AttributeValue, Element, Event, Properties, TaskId,
};
use std::{
    any::Any,
    cell::{Ref, RefCell},
    fmt::{Arguments, Debug},
    future::Future,
    rc::Rc,
    sync::Arc,
};

/// A component's unique identifier.
///
/// `ScopeId` is a `usize` that acts a key for the internal slab of Scopes. This means that the key is not unqiue across
/// time. We do try and guarantee that between calls to `wait_for_work`, no ScopeIds will be recycled in order to give
/// time for any logic that relies on these IDs to properly update.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ScopeId(pub usize);

impl ScopeId {
    /// The root ScopeId.
    ///
    /// This scope will last for the entire duration of your app, making it convenient for long-lived state
    /// that is created dynamically somewhere down the component tree.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// use dioxus_signals::*;
    /// let my_persistent_state = Signal::new_in_scope(ScopeId::ROOT, String::new());
    /// ```
    pub const ROOT: ScopeId = ScopeId(0);
}

/// A component's state separate from its props.
///
/// This struct exists to provide a common interface for all scopes without relying on generics.
pub struct ScopeState {
    pub(crate) runtime: Rc<Runtime>,
    pub(crate) context_id: ScopeId,

    pub(crate) last_rendered_node: Option<RenderReturn>,

    pub(crate) props: BoxedAnyProps,
}

impl Drop for ScopeState {
    fn drop(&mut self) {
        self.runtime.remove_context(self.context_id);
    }
}

impl<'src> ScopeState {
    pub(crate) fn context(&self) -> Ref<'_, ScopeContext> {
        self.runtime.get_context(self.context_id).unwrap()
    }

    /// Get the name of this component
    pub fn name(&self) -> &str {
        self.context().name
    }

    /// Get a handle to the currently active head node arena for this Scope
    ///
    /// This is useful for traversing the tree outside of the VirtualDom, such as in a custom renderer or in SSR.
    ///
    /// Panics if the tree has not been built yet.
    pub fn root_node(&self) -> &RenderReturn {
        self.try_root_node()
            .expect("The tree has not been built yet. Make sure to call rebuild on the tree before accessing its nodes.")
    }

    /// Try to get a handle to the currently active head node arena for this Scope
    ///
    /// This is useful for traversing the tree outside of the VirtualDom, such as in a custom renderer or in SSR.
    ///
    /// Returns [`None`] if the tree has not been built yet.
    pub fn try_root_node(&self) -> Option<&RenderReturn> {
        self.last_rendered_node.as_ref()
    }

    /// Get the height of this Scope - IE the number of scopes above it.
    ///
    /// A Scope with a height of `0` is the root scope - there are no other scopes above it.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new(|cx|  cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    ///
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.height(), 0);
    /// ```
    pub fn height(&self) -> u32 {
        self.context().height
    }

    /// Get the Parent of this [`Scope`] within this Dioxus [`crate::VirtualDom`].
    ///
    /// This ID is not unique across Dioxus [`crate::VirtualDom`]s or across time. IDs will be reused when components are unmounted.
    ///
    /// The base component will not have a parent, and will return `None`.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new(|cx|  cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    ///
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.parent(), None);
    /// ```
    pub fn parent(&self) -> Option<ScopeId> {
        // safety: the pointer to our parent is *always* valid thanks to the bump arena
        self.context().parent_id()
    }

    /// Get the ID of this Scope within this Dioxus [`crate::VirtualDom`].
    ///
    /// This ID is not unique across Dioxus [`crate::VirtualDom`]s or across time. IDs will be reused when components are unmounted.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new(|cx|  cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.scope_id(), 0);
    /// ```
    pub fn scope_id(&self) -> ScopeId {
        self.context().scope_id()
    }

    /// Create a subscription that schedules a future render for the reference component
    ///
    /// ## Notice: you should prefer using [`Self::schedule_update_any`] and [`Self::scope_id`]
    pub fn schedule_update(&self) -> Arc<dyn Fn() + Send + Sync + 'static> {
        self.context().schedule_update()
    }

    /// Schedule an update for any component given its [`ScopeId`].
    ///
    /// A component's [`ScopeId`] can be obtained from `use_hook` or the [`ScopeState::scope_id`] method.
    ///
    /// This method should be used when you want to schedule an update for a component
    pub fn schedule_update_any(&self) -> Arc<dyn Fn(ScopeId) + Send + Sync> {
        self.context().schedule_update_any()
    }

    /// Mark this scope as dirty, and schedule a render for it.
    pub fn needs_update(&self) {
        self.context().needs_update()
    }

    /// Get the [`ScopeId`] of a mounted component.
    ///
    /// `ScopeId` is not unique for the lifetime of the [`crate::VirtualDom`] - a [`ScopeId`] will be reused if a component is unmounted.
    pub fn needs_update_any(&self, id: ScopeId) {
        self.context().needs_update_any(id)
    }

    /// Return any context of type T if it exists on this scope
    pub fn has_context<T: 'static + Clone>(&self) -> Option<T> {
        self.context().has_context()
    }

    /// Try to retrieve a shared state with type `T` from any parent scope.
    ///
    /// Clones the state if it exists.
    pub fn consume_context<T: 'static + Clone>(&self) -> Option<T> {
        self.context().consume_context()
    }

    /// Expose state to children further down the [`crate::VirtualDom`] Tree. Requires `Clone` on the context to allow getting values down the tree.
    ///
    /// This is a "fundamental" operation and should only be called during initialization of a hook.
    ///
    /// For a hook that provides the same functionality, use `use_provide_context` and `use_context` instead.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// struct SharedState(&'static str);
    ///
    /// static App: Component = |cx| {
    ///     cx.use_hook(|| cx.provide_context(SharedState("world")));
    ///     render!(Child {})
    /// }
    ///
    /// static Child: Component = |cx| {
    ///     let state = cx.consume_state::<SharedState>();
    ///     render!(div { "hello {state.0}" })
    /// }
    /// ```
    pub fn provide_context<T: 'static + Clone>(&self, value: T) -> T {
        self.context().provide_context(value)
    }

    /// Provide a context to the root and then consume it
    ///
    /// This is intended for "global" state management solutions that would rather be implicit for the entire app.
    /// Things like signal runtimes and routers are examples of "singletons" that would benefit from lazy initialization.
    ///
    /// Note that you should be checking if the context existed before trying to provide a new one. Providing a context
    /// when a context already exists will swap the context out for the new one, which may not be what you want.
    pub fn provide_root_context<T: 'static + Clone>(&self, context: T) -> T {
        self.context().provide_root_context(context)
    }

    /// Pushes the future onto the poll queue to be polled after the component renders.
    pub fn push_future(&self, fut: impl Future<Output = ()> + 'static) -> TaskId {
        self.context().push_future(fut)
    }

    /// Spawns the future but does not return the [`TaskId`]
    pub fn spawn(&self, fut: impl Future<Output = ()> + 'static) {
        self.context().spawn(fut);
    }

    /// Spawn a future that Dioxus won't clean up when this component is unmounted
    ///
    /// This is good for tasks that need to be run after the component has been dropped.
    pub fn spawn_forever(&self, fut: impl Future<Output = ()> + 'static) -> TaskId {
        self.context().spawn_forever(fut)
    }

    /// Informs the scheduler that this task is no longer needed and should be removed.
    ///
    /// This drops the task immediately.
    pub fn remove_future(&self, id: TaskId) {
        self.context().remove_future(id);
    }

    /// Create a dynamic text node using [`Arguments`] and the [`ScopeState`]'s internal [`Bump`] allocator
    pub fn text_node(&'src self, args: Arguments) -> DynamicNode {
        DynamicNode::Text(VText {
            value: args.to_string(),
            id: Default::default(),
        })
    }

    /// Convert any item that implements [`IntoDynNode`] into a [`DynamicNode`] using the internal [`Bump`] allocator
    pub fn make_node<'c, I>(&'src self, into: impl IntoDynNode<I> + 'c) -> DynamicNode {
        into.into_dyn_node()
    }

    /// Create a new [`Attribute`] from a name, value, namespace, and volatile bool
    ///
    /// "Volatile" referes to whether or not Dioxus should always override the value. This helps prevent the UI in
    /// some renderers stay in sync with the VirtualDom's understanding of the world
    pub fn attr(
        &'src self,
        name: &'static str,
        value: impl IntoAttributeValue,
        namespace: Option<&'static str>,
        volatile: bool,
    ) -> Attribute {
        Attribute {
            name,
            namespace,
            volatile,
            mounted_element: Default::default(),
            value: value.into_value(),
        }
    }

    /// Create a new [`DynamicNode::Component`] variant
    ///
    ///
    /// The given component can be any of four signatures. Remember that an [`Element`] is really a [`Result<VNode>`].
    ///
    /// ```rust, ignore
    /// // Without explicit props
    /// fn(Scope) -> Element;
    /// async fn(Scope<'_>) -> Element;
    ///
    /// // With explicit props
    /// fn(Scope<Props>) -> Element;
    /// async fn(Scope<Props<'_>>) -> Element;
    /// ```
    pub fn component<P>(
        &self,
        component: fn(P) -> Element,
        props: P,
        fn_name: &'static str,
    ) -> DynamicNode
    where
        // The properties must be valid until the next bump frame
        P: Properties,
    {
        let vcomp = VProps::new(component, P::memoize, props, fn_name);

        DynamicNode::Component(VComponent {
            name: fn_name,
            render_fn: component as *const (),
            props: BoxedAnyProps::new(vcomp),
            scope: Default::default(),
        })
    }

    /// Create a new [`EventHandler`] from an [`FnMut`]
    pub fn event_handler<T>(&self, mut f: impl FnMut(T) + 'static) -> EventHandler<T> {
        let callback = RefCell::new(Some(Box::new(move |event: T| {
            f(event);
        }) as Box<dyn FnMut(T)>));
        EventHandler {
            callback,
            origin: self.context().id,
        }
    }

    /// Create a new [`AttributeValue`] with the listener variant from a callback
    ///
    /// The callback must be confined to the lifetime of the ScopeState
    pub fn listener<T: 'static>(
        &self,
        mut callback: impl FnMut(Event<T>) + 'static,
    ) -> AttributeValue {
        AttributeValue::Listener(RefCell::new(Box::new(move |event: Event<dyn Any>| {
            if let Ok(data) = event.data.downcast::<T>() {
                callback(Event {
                    propagates: event.propagates,
                    data,
                });
            }
        })))
    }

    /// Create a new [`AttributeValue`] with a value that implements [`AnyValue`]
    pub fn any_value<T: AnyValue>(&'src self, value: T) -> AttributeValue {
        AttributeValue::Any(Box::new(value))
    }

    /// Mark this component as suspended and then return None
    pub fn suspend(&self) -> Option<Element> {
        let cx = self.context();
        cx.suspend();
        None
    }
}
