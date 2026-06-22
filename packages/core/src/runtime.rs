use crate::mount::Mount;
use crate::scheduler::ScopeOrder;
use crate::scope_context::SuspenseLocation;
use crate::{
    AttributeValue, DynamicNode, ElementId, Event, RenderTargetId, VNode, innerlude::MountId,
};
use crate::{CapturedError, arena::RenderTargetState};
use crate::{
    SuspenseContext,
    innerlude::{DirtyTasks, Effect},
};
use crate::{
    Task,
    innerlude::{LocalTask, SchedulerMsg},
    scope_context::Scope,
    scopes::ScopeId,
};
use generational_box::{AnyStorage, Owner, SyncStorage, UnsyncStorage};
use rustc_hash::FxHashSet;
use slab::Slab;
use slotmap::DefaultKey;
use std::any::Any;
use std::collections::BTreeSet;
use std::{
    cell::{Cell, Ref, RefCell},
    rc::Rc,
};
use tracing::instrument;

use dioxus_core_template::{TemplatePath, TemplateSlotTarget};

#[derive(Clone, Copy)]
struct EventTarget {
    mount: MountId,
    path: EventTargetPath,
}

#[derive(Clone, Copy)]
enum EventTargetPath {
    Static(TemplatePath),
    Slot(TemplateSlotTarget),
}

impl EventTargetPath {
    fn is_under_attr(self, attr: TemplatePath) -> bool {
        match self {
            Self::Static(path) => path.starts_with(attr),
            Self::Slot(TemplateSlotTarget::BeforeStatic(path)) => {
                path.split_insertion().0.starts_with(attr)
            }
            Self::Slot(TemplateSlotTarget::AppendChildren(path)) => path.starts_with(attr),
        }
    }

    fn is_exact_static(self, attr: TemplatePath) -> bool {
        matches!(self, Self::Static(path) if path == attr)
    }
}

thread_local! {
    static RUNTIMES: RefCell<Vec<Rc<Runtime>>> = const { RefCell::new(vec![]) };
}

#[derive(Clone, Copy)]
struct ScopeStackFrame {
    scope: ScopeId,
    target_id: RenderTargetId,
}

/// A global runtime that is shared across all scopes that provides the async runtime and context API
pub struct Runtime {
    // We use this to track the current scope
    // This stack should only be modified through [`Runtime::with_scope_on_stack`] to ensure that the stack is correctly restored
    scope_stack: RefCell<Vec<ScopeStackFrame>>,

    current_render_target: Cell<RenderTargetId>,

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

    // The renderer targets and their element id arenas.
    pub(crate) render_targets: RefCell<Slab<RenderTargetState>>,

    // Once nodes are mounted, their persistent mount identity is stored here.
    // Each mount is associated with a whole rsx block. [`Runtime::elements`]
    // link to a specific node in that block.
    pub(crate) mounts: RefCell<Slab<Mount>>,

    // Mounts that the in-progress diff has moved or replaced and whose committed
    // position is therefore stale. Placement scans consult this set so they never
    // anchor against a node that is mid-move. It is empty between diffs.
    placement_stale: RefCell<FxHashSet<MountId>>,
}

struct ScopeStackGuard<'a> {
    runtime: &'a Runtime,
}

impl Drop for ScopeStackGuard<'_> {
    fn drop(&mut self) {
        self.runtime.pop_scope();
    }
}

struct SuspenseLocationGuard<'a> {
    runtime: &'a Runtime,
}

impl Drop for SuspenseLocationGuard<'_> {
    fn drop(&mut self) {
        self.runtime.suspense_stack.borrow_mut().pop();
    }
}

impl Runtime {
    pub(crate) fn new(sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>) -> Rc<Self> {
        let mut render_targets = Slab::default();
        let root = render_targets.insert(RenderTargetState::new());
        debug_assert_eq!(root, RenderTargetId::ROOT.index());

        Rc::new(Self {
            sender,
            rendering: Cell::new(false),
            scope_states: Default::default(),
            scope_stack: Default::default(),
            current_render_target: Cell::new(RenderTargetId::ROOT),
            suspense_stack: Default::default(),
            current_task: Default::default(),
            tasks: Default::default(),
            suspended_tasks: Default::default(),
            pending_effects: Default::default(),
            dirty_tasks: Default::default(),
            render_targets: RefCell::new(render_targets),
            mounts: Default::default(),
            placement_stale: Default::default(),
        })
    }

    /// Mark a mount as having a stale committed position for the duration of the
    /// active diff, so placement scans skip it. O(1). Cleared by
    /// [`Runtime::unmark_placement_stale`] once the diff that moved it commits.
    pub(crate) fn mark_placement_stale(&self, mount: MountId) {
        self.placement_stale.borrow_mut().insert(mount);
    }

    /// Clear a stale marker once the mount's new position is committed. O(1).
    pub(crate) fn unmark_placement_stale(&self, mount: MountId) {
        self.placement_stale.borrow_mut().remove(&mount);
    }

    /// Whether `mount`'s committed position is stale and must not anchor an
    /// insertion. O(1).
    pub(crate) fn is_placement_stale(&self, mount: MountId) -> bool {
        self.placement_stale.borrow().contains(&mount)
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

    /// Get the render target currently receiving renderer mutations: the
    /// target of the scope currently being rendered, or the root target when
    /// no scope is active. Every scope carries a flat target assignment;
    /// portal scopes carry the portal's target and their subtree inherits it.
    pub(crate) fn current_render_target_id(&self) -> RenderTargetId {
        self.current_render_target.get()
    }

    /// Create a new renderer target with an isolated [`ElementId`](crate::ElementId) arena.
    ///
    /// Hosts serve render targets through [`MultiWriter`](crate::MultiWriter)
    /// implementations passed into [`VirtualDom::rebuild`](crate::VirtualDom::rebuild)
    /// and [`VirtualDom::render_immediate`](crate::VirtualDom::render_immediate). If
    /// the host does not serve a writer for the target, those mutations are skipped.
    pub fn create_render_target(&self) -> RenderTargetId {
        let mut targets = self.render_targets.borrow_mut();
        RenderTargetId::new(targets.insert(RenderTargetState::new()))
    }

    /// Remove a render target previously created with [`create_render_target`](Self::create_render_target).
    ///
    /// This drops the target's [`ElementId`](crate::ElementId) arena and template
    /// cache, freeing the slot so its [`RenderTargetId`] may be handed back out by
    /// a later [`create_render_target`](Self::create_render_target). The root target
    /// ([`RenderTargetId::ROOT`]) is permanent and is never removed.
    ///
    /// The caller must ensure nothing renders into the target anymore: every node
    /// mounted into it — for example the [`Portal`](crate::Portal) feeding it — must
    /// already have been removed from the tree. Removing a target that still has
    /// live mounts leaves those mounts dangling and panics on the next render that
    /// touches the target.
    ///
    /// Returns `true` if a target was removed, or `false` if `id` was the root or
    /// referred to a target that was already gone.
    pub fn remove_render_target(&self, id: RenderTargetId) -> bool {
        if id == RenderTargetId::ROOT {
            return false;
        }
        let mut targets = self.render_targets.borrow_mut();
        #[cfg(debug_assertions)]
        if let Some(target) = targets.get(id.index()) {
            debug_assert!(
                target.elements.iter().all(|(_, slot)| slot.is_none()),
                "removing render target {id:?} while it still has live mounted elements"
            );
        }
        targets.try_remove(id.index()).is_some()
    }

    /// Create a scope context. This slab is synchronized with the scope slab.
    pub(crate) fn create_scope(&self, context: Scope) {
        let id = context.id.index();
        let mut scopes = self.scope_states.borrow_mut();
        if id == scopes.len() {
            scopes.push(Some(context));
            return;
        }

        if scopes.len() <= id {
            scopes.resize_with(id + 1, Default::default);
        }
        scopes[id] = Some(context);
    }

    pub(crate) fn set_scope_target_id(&self, scope: ScopeId, target_id: RenderTargetId) {
        {
            let scope_state = self.get_state(scope);
            scope_state.set_target_id(target_id);
        }

        {
            let mut scope_stack = self.scope_stack.borrow_mut();
            if let Some(current) = scope_stack.last_mut()
                && current.scope == scope
            {
                current.target_id = target_id;
                self.current_render_target.set(target_id);
            }
        }
    }

    pub(crate) fn remove_scope(self: &Rc<Self>, id: ScopeId) {
        {
            let borrow = self.scope_states.borrow();
            if let Some(scope) = &borrow[id.index()] {
                let has_scoped_state = !scope.spawned_tasks.borrow().is_empty()
                    || !scope.hooks.borrow().is_empty()
                    || has_user_shared_contexts(scope);

                if has_scoped_state {
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
                } else {
                    // Empty scopes do not need the runtime/scope stack just to clear bookkeeping.
                    self.pending_effects
                        .borrow_mut()
                        .remove(&ScopeOrder::new(scope.height, scope.id));
                }
            }
        }
        // Bind the removed scope so the `borrow_mut()` temporary is released at the end of
        // this statement, *before* the scope is dropped. Otherwise the scope's `Drop` runs
        // while `scope_states` is still mutably borrowed and re-entrant calls into
        // `get_state` panic with "already mutably borrowed".
        let removed = self.scope_states.borrow_mut()[id.index()].take();
        drop(removed);
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
        self.scope_stack
            .borrow()
            .last()
            .map(|frame| frame.scope)
            .unwrap()
    }

    /// Try to get the current scope id, returning None if it we aren't actively inside a scope
    pub fn try_current_scope_id(&self) -> Option<ScopeId> {
        self.scope_stack.borrow().last().map(|frame| frame.scope)
    }

    /// Call this function with the current scope set to the given scope
    #[track_caller]
    pub fn in_scope<O>(self: &Rc<Self>, id: ScopeId, f: impl FnOnce() -> O) -> O {
        let _runtime_guard = RuntimeGuard::new(self.clone());
        self.with_scope_on_stack(id, f)
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
        let _guard = SuspenseLocationGuard { runtime: self };
        f()
    }

    /// Run a callback with the current scope at the top of the stack
    pub(crate) fn with_scope_on_stack<O>(&self, scope: ScopeId, f: impl FnOnce() -> O) -> O {
        self.push_scope(scope);
        let _guard = ScopeStackGuard { runtime: self };
        f()
    }

    /// Push a scope onto the stack
    fn push_scope(&self, scope: ScopeId) {
        let (suspense_location, target_id) = self
            .scope_states
            .borrow()
            .get(scope.index())
            .and_then(|s| s.as_ref())
            .map(|s| (s.suspense_location(), s.target_id()))
            .unwrap_or((SuspenseLocation::default(), RenderTargetId::ROOT));
        self.suspense_stack.borrow_mut().push(suspense_location);
        self.scope_stack
            .borrow_mut()
            .push(ScopeStackFrame { scope, target_id });
        self.current_render_target.set(target_id);
    }

    /// Pop a scope off the stack
    fn pop_scope(&self) {
        self.suspense_stack.borrow_mut().pop();
        let target_id = {
            let mut scope_stack = self.scope_stack.borrow_mut();
            scope_stack.pop();
            scope_stack
                .last()
                .map_or(RenderTargetId::ROOT, |frame| frame.target_id)
        };
        self.current_render_target.set(target_id);
    }

    /// Get the state for any scope given its ID
    ///
    /// This is useful for inserting or removing contexts from a scope, or rendering out its root node
    pub(crate) fn get_state(&self, id: ScopeId) -> Ref<'_, Scope> {
        Ref::filter_map(self.scope_states.borrow(), |scopes| {
            scopes.get(id.index()).and_then(|f| f.as_ref())
        })
        .ok()
        .unwrap()
    }

    /// Get the state for any scope given its ID
    ///
    /// This is useful for inserting or removing contexts from a scope, or rendering out its root node
    pub(crate) fn try_get_state(&self, id: ScopeId) -> Option<Ref<'_, Scope>> {
        Ref::filter_map(self.scope_states.borrow(), |contexts| {
            contexts.get(id.index()).and_then(|f| f.as_ref())
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
        let scopes = self.scope_states.borrow();
        let scope = &scopes[scope_id.index()].as_ref().unwrap();
        let location = scope.suspense_location();
        if self.suspended_tasks.get() == 0 {
            return !matches!(
                location,
                SuspenseLocation::UnderSuspense { boundary, .. } if boundary.is_suspended()
            );
        }
        location.should_write()
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
        self.handle_event_for_target(RenderTargetId::ROOT, name, event, element);
    }

    /// Call a listener inside the VirtualDom with data from a specific render target.
    ///
    /// `ElementId`s are renderer-local, so multi-target renderers should use this
    /// method instead of [`Self::handle_event`].
    #[instrument(
        skip(self, event),
        level = "trace",
        name = "Runtime::handle_event_for_target"
    )]
    pub fn handle_event_for_target(
        self: &Rc<Self>,
        target_id: RenderTargetId,
        name: &str,
        event: Event<dyn Any>,
        element: ElementId,
    ) {
        let _runtime = RuntimeGuard::new(self.clone());
        let targets = self.render_targets.borrow();
        let Some(target) = targets.get(target_id.index()) else {
            return;
        };

        let parent_ref = target.elements.get(element.index()).copied().flatten();
        drop(targets);

        if let Some(parent_ref) = parent_ref
            && let Some(path) = self.event_target_path(parent_ref.mount, element)
        {
            let target = EventTarget {
                mount: parent_ref.mount,
                path,
            };
            if event.propagates() {
                self.handle_bubbling_event(target, name, event);
            } else {
                self.handle_non_bubbling_event(target, name, event);
            }
        }
    }

    fn event_target_path(&self, mount_id: MountId, element: ElementId) -> Option<EventTargetPath> {
        let mounts = self.mounts.borrow();
        let mount = mounts.get(mount_id.0)?;
        let node = mount.node();

        for group in node.dynamic_attributes() {
            let path = group.static_path();
            let Some(id) = mount.mounted_anchor_node(group.anchor_index()) else {
                continue;
            };
            if id.element_id() == element {
                return Some(EventTargetPath::Static(path));
            }
        }

        None
    }

    fn child_slot_path(
        &self,
        parent_mount: MountId,
        child_mount: MountId,
    ) -> Option<EventTargetPath> {
        let mounts = self.mounts.borrow();
        let parent = mounts.get(parent_mount.0)?;
        let parent_node = parent.node();

        for group in parent_node.dynamic_nodes() {
            let target = group.slot_target();
            for idx in group.ids() {
                match parent_node.dynamic_values[idx].node() {
                    DynamicNode::Fragment(children) => {
                        if children.is_empty() {
                            continue;
                        }
                        if parent
                            .non_empty_fragment_children(idx, children.len())
                            .contains(&child_mount)
                        {
                            return Some(EventTargetPath::Slot(target));
                        }
                    }
                    DynamicNode::Component(_) => {
                        let Some(scope) = parent.component_scope(idx) else {
                            continue;
                        };
                        let root_mount = self
                            .scope_states
                            .borrow()
                            .get(scope.index())
                            .and_then(|scope| scope.as_ref())
                            .and_then(|scope| scope.root_mount());
                        if root_mount == Some(child_mount) {
                            return Some(EventTargetPath::Slot(target));
                        }
                    }
                    DynamicNode::Text(_) => {}
                }
            }
        }

        None
    }

    fn visit_event_attributes(
        node: &VNode,
        target_path: EventTargetPath,
        name: &str,
        path_matches: impl Fn(EventTargetPath, TemplatePath) -> bool,
        mut visit: impl FnMut(&AttributeValue, TemplatePath) -> bool,
    ) {
        for group in node.dynamic_attributes() {
            let attr_path = group.static_path();
            if !path_matches(target_path, attr_path) {
                continue;
            }

            for idx in group.ids() {
                let attrs = node.dynamic_values[idx].attrs();
                for attr in attrs.iter() {
                    // Remove the "on" prefix if it exists, TODO, we should remove this and settle on one
                    if attr.name.get(2..) == Some(name) && visit(&attr.value, attr_path) {
                        break;
                    }
                }
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
        skip(self, parent, uievent),
        level = "trace",
        name = "VirtualDom::handle_bubbling_event"
    )]
    fn handle_bubbling_event(&self, parent: EventTarget, name: &str, uievent: Event<dyn Any>) {
        // If the event bubbles, we traverse through the tree until we find the target element.
        // Loop through each dynamic attribute (in a depth first order) in this template before moving up to the template's parent.
        let mut parent = Some(parent);
        while let Some(target) = parent {
            let mut listeners = vec![];
            let logical_parent;

            // We do this in its own block to prevent mount borrows from staying open while we call user code
            {
                let mounts = self.mounts.borrow();
                let Some(mount) = mounts.get(target.mount.0) else {
                    // If the node is suspended and not mounted, we can just ignore the event
                    return;
                };

                let el_ref = mount.node();
                let target_path = target.path;
                logical_parent = mount.logical_parent();

                // Accumulate listeners into the listener list bottom to top
                Self::visit_event_attributes(
                    el_ref,
                    target_path,
                    name,
                    EventTargetPath::is_under_attr,
                    |value, attr_path| {
                        if let AttributeValue::Listener(listener) = value {
                            listeners.push((attr_path, listener.clone()));
                        }

                        // Break if this is the exact target element.
                        // This means we won't call two listeners with the same name on the same element. This should be
                        // documented, or be rejected from the rsx! macro outright
                        target_path.is_exact_static(attr_path)
                    },
                );
            }

            // Now that we've accumulated all the parent attributes for the target element, call them in reverse order
            // We check the bubble state between each call to see if the event has been stopped from bubbling
            tracing::event!(
                tracing::Level::TRACE,
                "Calling {} listeners",
                listeners.len()
            );
            listeners.sort_by_key(|(path, _)| std::cmp::Reverse(path.depth()));
            for (_, listener) in listeners {
                listener.call(uievent.clone());
                let metadata = uievent.metadata.borrow();

                if !metadata.propagates {
                    return;
                }
            }

            parent = logical_parent.and_then(|parent_ref| {
                self.child_slot_path(parent_ref.mount, target.mount)
                    .map(|path| EventTarget {
                        mount: parent_ref.mount,
                        path,
                    })
            });
        }
    }

    /// Call an event listener in the simplest way possible without bubbling upwards
    #[instrument(
        skip(self, node, uievent),
        level = "trace",
        name = "VirtualDom::handle_non_bubbling_event"
    )]
    fn handle_non_bubbling_event(&self, node: EventTarget, name: &str, uievent: Event<dyn Any>) {
        let listeners = {
            let mounts = self.mounts.borrow();
            let Some(mount) = mounts.get(node.mount.0) else {
                // If the node is suspended and not mounted, we can just ignore the event
                return;
            };
            let mut listeners = Vec::new();
            let target_path = node.path;

            Self::visit_event_attributes(
                mount.node(),
                target_path,
                name,
                EventTargetPath::is_exact_static,
                |value, _| {
                    if let AttributeValue::Listener(listener) = value {
                        listeners.push(listener.clone());
                        true
                    } else {
                        false
                    }
                },
            );

            listeners
        };

        for listener in listeners {
            listener.call(uievent.clone());
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
        self.get_state(scope).needs_update_any(scope);
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
                scope.needs_update_any(scope.id);
            }
        });
    }

    /// Check if the virtual dom is currently rendering
    pub fn vdom_is_rendering(&self) -> bool {
        self.rendering.get()
    }
}

fn has_user_shared_contexts(scope: &Scope) -> bool {
    scope.shared_contexts.borrow().iter().any(|context| {
        let context = context.as_ref();
        !context.is::<Owner<SyncStorage>>() && !context.is::<Owner<UnsyncStorage>>()
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> Rc<Runtime> {
        let (sender, _receiver) = futures_channel::mpsc::unbounded();
        Runtime::new(sender)
    }

    fn catch_expected_panic(f: impl FnOnce()) {
        let panic_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        std::panic::set_hook(panic_hook);
        assert!(result.is_err());
    }

    #[test]
    fn with_scope_on_stack_restores_after_panic() {
        let runtime = runtime();

        catch_expected_panic(|| {
            runtime.with_scope_on_stack(ScopeId::new(7), || panic!("forced panic"));
        });

        assert_eq!(runtime.try_current_scope_id(), None);
        assert!(runtime.current_suspense_location().is_none());
    }

    #[test]
    fn with_suspense_location_restores_after_panic() {
        let runtime = runtime();

        catch_expected_panic(|| {
            runtime.with_suspense_location(SuspenseLocation::default(), || panic!("forced panic"));
        });

        assert!(runtime.current_suspense_location().is_none());
    }
}
