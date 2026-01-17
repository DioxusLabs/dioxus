use crate::nodes::VNodeMount;
use crate::scheduler::ScopeOrder;
use crate::scope_context::SuspenseLocation;
use crate::{arena::ElementRef, CapturedError};
use crate::{
    innerlude::{DirtyTasks, Effect},
    SuspenseContext,
};
use crate::{
    innerlude::{LocalTask, SchedulerMsg},
    scope_context::Scope,
    scopes::ScopeId,
    Task,
};
use crate::{AttributeValue, ElementId, Event};
use generational_box::{AnyStorage, Owner};
use slab::Slab;
use slotmap::DefaultKey;
use std::any::Any;
use std::collections::BTreeSet;
use std::{
    cell::{Cell, Ref, RefCell},
    rc::Rc,
};
use tracing::instrument;

thread_local! {
    static RUNTIMES: RefCell<Vec<Rc<Runtime>>> = const { RefCell::new(vec![]) };
}

/// A global runtime that is shared across all scopes that provides the async runtime and context API
pub struct Runtime {
    // We use this to track the current scope
    // This stack should only be modified through [`Runtime::with_scope_on_stack`] to ensure that the stack is correctly restored
    scope_stack: RefCell<Vec<ScopeId>>,

    // We use this to track the current suspense location. Generally this lines up with the scope stack, but it may be different for children of a suspense boundary
    // This stack should only be modified through [`Runtime::with_suspense_location`] to ensure that the stack is correctly restored
    suspense_stack: RefCell<Vec<SuspenseLocation>>,

    // A hand-rolled slab of scope states
    pub(crate) scope_states: RefCell<Vec<Option<Scope>>>,

    // We use this to track the current task
    pub(crate) current_task: Cell<Option<Task>>,

    /// Tasks created with cx.spawn
    pub(crate) tasks: RefCell<slotmap::SlotMap<DefaultKey, Rc<LocalTask>>>,

    // Currently suspended tasks
    pub(crate) suspended_tasks: Cell<usize>,

    pub(crate) rendering: Cell<bool>,

    pub(crate) sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,

    // The effects that need to be run after the next render
    pub(crate) pending_effects: RefCell<BTreeSet<Effect>>,

    // Tasks that are waiting to be polled
    pub(crate) dirty_tasks: RefCell<BTreeSet<DirtyTasks>>,

    // The element ids that are used in the renderer
    // These mark a specific place in a whole rsx block
    pub(crate) elements: RefCell<Slab<Option<ElementRef>>>,

    // Once nodes are mounted, the information about where they are mounted is stored here
    // We need to store this information on the virtual dom so that we know what nodes are mounted where when we bubble events
    // Each mount is associated with a whole rsx block. [`VirtualDom::elements`] link to a specific node in the block
    pub(crate) mounts: RefCell<Slab<VNodeMount>>,
}

impl Runtime {
    pub(crate) fn new(sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>) -> Rc<Self> {
        let mut elements = Slab::default();
        // the root element is always given element ID 0 since it's the container for the entire tree
        elements.insert(None);

        Rc::new(Self {
            sender,
            rendering: Cell::new(false),
            scope_states: Default::default(),
            scope_stack: Default::default(),
            suspense_stack: Default::default(),
            current_task: Default::default(),
            tasks: Default::default(),
            suspended_tasks: Default::default(),
            pending_effects: Default::default(),
            dirty_tasks: Default::default(),
            elements: RefCell::new(elements),
            mounts: Default::default(),
        })
    }

    /// Get the current runtime
    pub fn current() -> Rc<Self> {
        RUNTIMES
            .with(|stack| stack.borrow().last().cloned())
            .unwrap_or_else(|| {
                panic!(
                    "Must be called from inside a Dioxus runtime.

Help: Some APIs in dioxus require a global runtime to be present.
If you are calling one of these APIs from outside of a dioxus runtime
(typically in a web-sys closure or dynamic library), you will need to
grab the runtime from a scope that has it and then move it into your
new scope with a runtime guard.

For example, if you are trying to use dioxus apis from a web-sys
closure, you can grab the runtime from the scope it is created in:

```rust
use dioxus::prelude::*;
static COUNT: GlobalSignal<i32> = Signal::global(|| 0);

#[component]
fn MyComponent() -> Element {{
    use_effect(|| {{
        // Grab the runtime from the MyComponent scope
        let runtime = Runtime::current().expect(\"Components run in the Dioxus runtime\");
        // Move the runtime into the web-sys closure scope
        let web_sys_closure = Closure::new(|| {{
            // Then create a guard to provide the runtime to the closure
            let _guard = RuntimeGuard::new(runtime);
            // and run whatever code needs the runtime
            tracing::info!(\"The count is: {{COUNT}}\");
        }});
    }})
}}
```"
                )
            })
    }

    /// Try to get the current runtime, returning None if it doesn't exist (outside the context of a dioxus app)
    pub fn try_current() -> Option<Rc<Self>> {
        RUNTIMES.with(|stack| stack.borrow().last().cloned())
    }

    /// Wrap a closure so that it always runs in the runtime that is currently active
    pub fn wrap_closure<'a, I, O>(f: impl Fn(I) -> O + 'a) -> impl Fn(I) -> O + 'a {
        let current_runtime = Self::current();
        move |input| match current_runtime.try_current_scope_id() {
            Some(scope) => current_runtime.in_scope(scope, || f(input)),
            None => {
                let _runtime_guard = RuntimeGuard::new(current_runtime.clone());
                f(input)
            }
        }
    }

    /// Run a closure with the rendering flag set to true
    pub(crate) fn while_rendering<T>(&self, f: impl FnOnce() -> T) -> T {
        self.rendering.set(true);
        let result = f();
        self.rendering.set(false);
        result
    }

    /// Run a closure with the rendering flag set to false
    pub(crate) fn while_not_rendering<T>(&self, f: impl FnOnce() -> T) -> T {
        let previous = self.rendering.get();
        self.rendering.set(false);
        let result = f();
        self.rendering.set(previous);
        result
    }

    /// Create a scope context. This slab is synchronized with the scope slab.
    pub(crate) fn create_scope(&self, context: Scope) {
        let id = context.id;
        let mut scopes = self.scope_states.borrow_mut();
        if scopes.len() <= id.0 {
            scopes.resize_with(id.0 + 1, Default::default);
        }
        scopes[id.0] = Some(context);
    }

    pub(crate) fn remove_scope(self: &Rc<Self>, id: ScopeId) {
        {
            let borrow = self.scope_states.borrow();
            if let Some(scope) = &borrow[id.0] {
                // Manually drop tasks, hooks, and contexts inside of the runtime
                self.in_scope(id, || {
                    // Drop all spawned tasks - order doesn't matter since tasks don't rely on eachother
                    // In theory nested tasks might not like this
                    for id in scope.spawned_tasks.take() {
                        self.remove_task(id);
                    }

                    // Drop all queued effects
                    self.pending_effects
                        .borrow_mut()
                        .remove(&ScopeOrder::new(scope.height, scope.id));

                    // Drop all hooks in reverse order in case a hook depends on another hook.
                    for hook in scope.hooks.take().drain(..).rev() {
                        drop(hook);
                    }

                    // Drop all contexts
                    scope.shared_contexts.take();
                });
            }
        }
        self.scope_states.borrow_mut()[id.0].take();
    }

    /// Get the owner for the current scope.
    #[track_caller]
    pub fn current_owner<S: AnyStorage>(&self) -> Owner<S> {
        self.get_state(self.current_scope_id()).owner()
    }

    /// Get the owner for the current scope.
    #[track_caller]
    pub fn scope_owner<S: AnyStorage>(&self, scope: ScopeId) -> Owner<S> {
        self.get_state(scope).owner()
    }

    /// Get the current scope id
    pub fn current_scope_id(&self) -> ScopeId {
        self.scope_stack.borrow().last().copied().unwrap()
    }

    /// Try to get the current scope id, returning None if it we aren't actively inside a scope
    pub fn try_current_scope_id(&self) -> Option<ScopeId> {
        self.scope_stack.borrow().last().copied()
    }

    /// Call this function with the current scope set to the given scope
    #[track_caller]
    pub fn in_scope<O>(self: &Rc<Self>, id: ScopeId, f: impl FnOnce() -> O) -> O {
        let _runtime_guard = RuntimeGuard::new(self.clone());
        {
            self.push_scope(id);
        }
        let o = f();
        {
            self.pop_scope();
        }
        o
    }

    /// Get the current suspense location
    pub(crate) fn current_suspense_location(&self) -> Option<SuspenseLocation> {
        self.suspense_stack.borrow().last().cloned()
    }

    /// Run a callback a [`SuspenseLocation`] at the top of the stack
    pub(crate) fn with_suspense_location<O>(
        &self,
        suspense_location: SuspenseLocation,
        f: impl FnOnce() -> O,
    ) -> O {
        self.suspense_stack.borrow_mut().push(suspense_location);
        let o = f();
        self.suspense_stack.borrow_mut().pop();
        o
    }

    /// Run a callback with the current scope at the top of the stack
    pub(crate) fn with_scope_on_stack<O>(&self, scope: ScopeId, f: impl FnOnce() -> O) -> O {
        self.push_scope(scope);
        let o = f();
        self.pop_scope();
        o
    }

    /// Push a scope onto the stack
    fn push_scope(&self, scope: ScopeId) {
        let suspense_location = self
            .scope_states
            .borrow()
            .get(scope.0)
            .and_then(|s| s.as_ref())
            .map(|s| s.suspense_location())
            .unwrap_or_default();
        self.suspense_stack.borrow_mut().push(suspense_location);
        self.scope_stack.borrow_mut().push(scope);
    }

    /// Pop a scope off the stack
    fn pop_scope(&self) {
        self.scope_stack.borrow_mut().pop();
        self.suspense_stack.borrow_mut().pop();
    }

    /// Get the state for any scope given its ID
    ///
    /// This is useful for inserting or removing contexts from a scope, or rendering out its root node
    pub(crate) fn get_state(&self, id: ScopeId) -> Ref<'_, Scope> {
        Ref::filter_map(self.scope_states.borrow(), |scopes| {
            scopes.get(id.0).and_then(|f| f.as_ref())
        })
        .ok()
        .unwrap()
    }

    /// Get the state for any scope given its ID
    ///
    /// This is useful for inserting or removing contexts from a scope, or rendering out its root node
    pub(crate) fn try_get_state(&self, id: ScopeId) -> Option<Ref<'_, Scope>> {
        Ref::filter_map(self.scope_states.borrow(), |contexts| {
            contexts.get(id.0).and_then(|f| f.as_ref())
        })
        .ok()
    }

    /// Pushes a new scope onto the stack
    pub(crate) fn push(runtime: Rc<Runtime>) {
        RUNTIMES.with(|stack| stack.borrow_mut().push(runtime));
    }

    /// Pops a scope off the stack
    pub(crate) fn pop() {
        RUNTIMES.with(|stack| stack.borrow_mut().pop().unwrap());
    }

    /// Runs a function with the current runtime
    pub(crate) fn with<R>(callback: impl FnOnce(&Runtime) -> R) -> R {
        callback(&Self::current())
    }

    /// Runs a function with the current scope
    pub(crate) fn with_current_scope<R>(callback: impl FnOnce(&Scope) -> R) -> R {
        Self::with(|rt| Self::with_scope(rt.current_scope_id(), callback))
    }

    /// Runs a function with the current scope
    pub(crate) fn with_scope<R>(scope: ScopeId, callback: impl FnOnce(&Scope) -> R) -> R {
        let rt = Runtime::current();
        Self::in_scope(&rt, scope, || callback(&rt.get_state(scope)))
    }

    /// Finish a render. This will mark all effects as ready to run and send the render signal.
    pub(crate) fn finish_render(&self) {
        // If there are new effects we can run, send a message to the scheduler to run them (after the renderer has applied the mutations)
        if !self.pending_effects.borrow().is_empty() {
            self.sender
                .unbounded_send(SchedulerMsg::EffectQueued)
                .expect("Scheduler should exist");
        }
    }

    /// Check if we should render a scope
    pub(crate) fn scope_should_render(&self, scope_id: ScopeId) -> bool {
        // If there are no suspended futures, we know the scope is not  and we can skip context checks
        if self.suspended_tasks.get() == 0 {
            return true;
        }

        // If this is not a suspended scope, and we are under a frozen context, then we should
        let scopes = self.scope_states.borrow();
        let scope = &scopes[scope_id.0].as_ref().unwrap();
        !matches!(scope.suspense_location(), SuspenseLocation::UnderSuspense(suspense) if suspense.is_suspended())
    }

    /// Call a listener inside the VirtualDom with data from outside the VirtualDom. **The ElementId passed in must be the id of an element with a listener, not a static node or a text node.**
    ///
    /// This method will identify the appropriate element. The data must match up with the listener declared. Note that
    /// this method does not give any indication as to the success of the listener call. If the listener is not found,
    /// nothing will happen.
    ///
    /// It is up to the listeners themselves to mark nodes as dirty.
    ///
    /// If you have multiple events, you can call this method multiple times before calling "render_with_deadline"
    #[instrument(skip(self, event), level = "trace", name = "Runtime::handle_event")]
    pub fn handle_event(self: &Rc<Self>, name: &str, event: Event<dyn Any>, element: ElementId) {
        let _runtime = RuntimeGuard::new(self.clone());
        let elements = self.elements.borrow();

        if let Some(Some(parent_path)) = elements.get(element.0).copied() {
            if event.propagates() {
                self.handle_bubbling_event(parent_path, name, event);
            } else {
                self.handle_non_bubbling_event(parent_path, name, event);
            }
        }
    }

    /*
    ------------------------
    The algorithm works by walking through the list of dynamic attributes, checking their paths, and breaking when
    we find the target path.

    With the target path, we try and move up to the parent until there is no parent.
    Due to how bubbling works, we call the listeners before walking to the parent.

    If we wanted to do capturing, then we would accumulate all the listeners and call them in reverse order.
    ----------------------

    For a visual demonstration, here we present a tree on the left and whether or not a listener is collected on the
    right.

    |           <-- yes (is ascendant)
    | | |       <-- no  (is not direct ascendant)
    | |         <-- yes (is ascendant)
    | | | | |   <--- target element, break early, don't check other listeners
    | | |       <-- no, broke early
    |           <-- no, broke early
    */
    #[instrument(
        skip(self, uievent),
        level = "trace",
        name = "VirtualDom::handle_bubbling_event"
    )]
    fn handle_bubbling_event(&self, parent: ElementRef, name: &str, uievent: Event<dyn Any>) {
        let mounts = self.mounts.borrow();

        // If the event bubbles, we traverse through the tree until we find the target element.
        // Loop through each dynamic attribute (in a depth first order) in this template before moving up to the template's parent.
        let mut parent = Some(parent);
        while let Some(path) = parent {
            let mut listeners = vec![];

            let Some(mount) = mounts.get(path.mount.0) else {
                // If the node is suspended and not mounted, we can just ignore the event
                return;
            };
            let el_ref = &mount.node;
            let node_template = el_ref.template;
            let target_path = path.path;

            // Accumulate listeners into the listener list bottom to top
            for (idx, this_path) in node_template.attr_paths.iter().enumerate() {
                let attrs = &*el_ref.dynamic_attrs[idx];

                for attr in attrs.iter() {
                    // Remove the "on" prefix if it exists, TODO, we should remove this and settle on one
                    if attr.name.get(2..) == Some(name) && target_path.is_descendant(this_path) {
                        listeners.push(&attr.value);

                        // Break if this is the exact target element.
                        // This means we won't call two listeners with the same name on the same element. This should be
                        // documented, or be rejected from the rsx! macro outright
                        if target_path == this_path {
                            break;
                        }
                    }
                }
            }

            // Now that we've accumulated all the parent attributes for the target element, call them in reverse order
            // We check the bubble state between each call to see if the event has been stopped from bubbling
            tracing::event!(
                tracing::Level::TRACE,
                "Calling {} listeners",
                listeners.len()
            );
            for listener in listeners.into_iter().rev() {
                if let AttributeValue::Listener(listener) = listener {
                    listener.call(uievent.clone());
                    let metadata = uievent.metadata.borrow();

                    if !metadata.propagates {
                        return;
                    }
                }
            }

            let mount = el_ref.mount.get().as_usize();
            parent = mount.and_then(|id| mounts.get(id).and_then(|el| el.parent));
        }
    }

    /// Call an event listener in the simplest way possible without bubbling upwards
    #[instrument(
        skip(self, uievent),
        level = "trace",
        name = "VirtualDom::handle_non_bubbling_event"
    )]
    fn handle_non_bubbling_event(&self, node: ElementRef, name: &str, uievent: Event<dyn Any>) {
        let mounts = self.mounts.borrow();
        let Some(mount) = mounts.get(node.mount.0) else {
            // If the node is suspended and not mounted, we can just ignore the event
            return;
        };
        let el_ref = &mount.node;
        let node_template = el_ref.template;
        let target_path = node.path;

        for (idx, this_path) in node_template.attr_paths.iter().enumerate() {
            let attrs = &*el_ref.dynamic_attrs[idx];

            for attr in attrs.iter() {
                // Remove the "on" prefix if it exists, TODO, we should remove this and settle on one
                // Only call the listener if this is the exact target element.
                if attr.name.get(2..) == Some(name) && target_path == this_path {
                    if let AttributeValue::Listener(listener) = &attr.value {
                        listener.call(uievent.clone());
                        break;
                    }
                }
            }
        }
    }

    /// Consume context from the current scope
    pub fn consume_context<T: 'static + Clone>(&self, id: ScopeId) -> Option<T> {
        self.get_state(id).consume_context::<T>()
    }

    /// Consume context from the current scope
    pub fn consume_context_from_scope<T: 'static + Clone>(&self, scope_id: ScopeId) -> Option<T> {
        self.get_state(scope_id).consume_context::<T>()
    }

    /// Check if the current scope has a context
    pub fn has_context<T: 'static + Clone>(&self, id: ScopeId) -> Option<T> {
        self.get_state(id).has_context::<T>()
    }

    /// Provide context to the current scope
    pub fn provide_context<T: 'static + Clone>(&self, id: ScopeId, value: T) -> T {
        self.get_state(id).provide_context(value)
    }

    /// Get the parent of the current scope if it exists
    pub fn parent_scope(&self, scope: ScopeId) -> Option<ScopeId> {
        self.get_state(scope).parent_id()
    }

    /// Check if the current scope is a descendant of the given scope
    pub fn is_descendant_of(&self, us: ScopeId, other: ScopeId) -> bool {
        let mut current = us;
        while let Some(parent) = self.parent_scope(current) {
            if parent == other {
                return true;
            }
            current = parent;
        }
        false
    }

    /// Mark the current scope as dirty, causing it to re-render
    pub fn needs_update(&self, scope: ScopeId) {
        self.get_state(scope).needs_update();
    }

    /// Get the height of the current scope
    pub fn height(&self, id: ScopeId) -> u32 {
        self.get_state(id).height
    }

    /// Throw a [`CapturedError`] into a scope. The error will bubble up to the nearest [`ErrorBoundary`](crate::ErrorBoundary) or the root of the app.
    ///
    /// # Examples
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// fn Component() -> Element {
    ///     let request = spawn(async move {
    ///         match reqwest::get("https://api.example.com").await {
    ///             Ok(_) => unimplemented!(),
    ///             // You can explicitly throw an error into a scope with throw_error
    ///             Err(err) => dioxus::core::Runtime::current().throw_error(ScopeId::APP, err),
    ///         }
    ///     });
    ///
    ///     unimplemented!()
    /// }
    /// ```
    pub fn throw_error(&self, id: ScopeId, error: impl Into<CapturedError> + 'static) {
        let error = error.into();
        if let Some(cx) = self.consume_context::<crate::ErrorContext>(id) {
            cx.insert_error(error)
        } else {
            tracing::error!(
                    "Tried to throw an error into an error boundary, but failed to locate a boundary: {:?}",
                    error
                )
        }
    }

    /// Get the suspense context the current scope is in
    pub fn suspense_context(&self) -> Option<SuspenseContext> {
        self.get_state(self.current_scope_id())
            .suspense_location()
            .suspense_context()
            .cloned()
    }

    /// Force every component to be dirty and require a re-render. Used by hot-reloading.
    ///
    /// This might need to change to a different flag in the event hooks order changes within components.
    /// What we really need is a way to mark components as needing a complete rebuild if they were hit by changes.
    pub fn force_all_dirty(&self) {
        self.scope_states.borrow_mut().iter().for_each(|state| {
            if let Some(scope) = state.as_ref() {
                scope.needs_update();
            }
        });
    }

    /// Check if the virtual dom is currently rendering
    pub fn vdom_is_rendering(&self) -> bool {
        self.rendering.get()
    }
}

/// A guard for a new runtime. This must be used to override the current runtime when importing components from a dynamic library that has it's own runtime.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_core::{Runtime, RuntimeGuard};
///
/// fn main() {
///     let virtual_dom = VirtualDom::new(app);
/// }
///
/// fn app() -> Element {
///     rsx! { Component { runtime: Runtime::current() } }
/// }
///
/// // In a dynamic library
/// #[derive(Props, Clone)]
/// struct ComponentProps {
///    runtime: std::rc::Rc<Runtime>,
/// }
///
/// impl PartialEq for ComponentProps {
///     fn eq(&self, _other: &Self) -> bool {
///         true
///     }
/// }
///
/// fn Component(cx: ComponentProps) -> Element {
///     use_hook(|| {
///         let _guard = RuntimeGuard::new(cx.runtime.clone());
///     });
///
///     rsx! { div {} }
/// }
/// ```
pub struct RuntimeGuard(());

impl RuntimeGuard {
    /// Create a new runtime guard that sets the current Dioxus runtime. The runtime will be reset when the guard is dropped
    pub fn new(runtime: Rc<Runtime>) -> Self {
        Runtime::push(runtime);
        Self(())
    }
}

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        Runtime::pop();
    }
}
