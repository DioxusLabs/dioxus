//! Public APIs for managing component state, tasks, and lifecycles.
//!
//! This module is separate from `Scope` to narrow what exactly is exposed to user code.
//!
//! We unsafely implement `send` for the VirtualDom, but those guarantees can only be

use crate::innerlude::*;
use std::{any::TypeId, ops::Deref, rc::Rc};

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
pub struct Context<'src> {
    pub scope: &'src Scope,
}

impl<'src> Copy for Context<'src> {}
impl<'src> Clone for Context<'src> {
    fn clone(&self) -> Self {
        Self { scope: self.scope }
    }
}

// We currently deref to props, but it might make more sense to deref to Scope?
// This allows for code that takes cx.xyz instead of cx.props.xyz
impl<'a> Deref for Context<'a> {
    type Target = &'a Scope;
    fn deref(&self) -> &Self::Target {
        &self.scope
    }
}

impl<'src> Context<'src> {
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
    /// const App: FC<()> = |(cx, props)|{
    ///     cx.render(rsx!{
    ///         CustomCard {
    ///             h1 {}
    ///             p {}
    ///         }
    ///     })
    /// }
    ///
    /// const CustomCard: FC<()> = |(cx, props)|{
    ///     cx.render(rsx!{
    ///         div {
    ///             h1 {"Title card"}
    ///             {cx.children()}
    ///         }
    ///     })
    /// }
    /// ```
    ///
    /// ## Notes:
    ///
    /// This method returns a "ScopeChildren" object. This object is copy-able and preserve the correct lifetime.
    pub fn children(&self) -> ScopeChildren<'src> {
        self.scope.child_nodes()
    }

    /// Create a subscription that schedules a future render for the reference component
    ///
    /// ## Notice: you should prefer using prepare_update and get_scope_id
    pub fn schedule_update(&self) -> Rc<dyn Fn() + 'static> {
        self.scope.memoized_updater.clone()
    }

    pub fn needs_update(&self) {
        (self.scope.memoized_updater)()
    }

    /// Schedule an update for any component given its ScopeId.
    ///
    /// A component's ScopeId can be obtained from `use_hook` or the [`Context::scope_id`] method.
    ///
    /// This method should be used when you want to schedule an update for a component
    pub fn schedule_update_any(&self) -> Rc<dyn Fn(ScopeId)> {
        self.scope.shared.schedule_any_immediate.clone()
    }

    /// Get the [`ScopeId`] of a mounted component.
    ///
    /// `ScopeId` is not unique for the lifetime of the VirtualDom - a ScopeId will be reused if a component is unmounted.
    pub fn scope_id(&self) -> ScopeId {
        self.scope.our_arena_idx
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
    ) -> DomTree<'src> {
        let bump = &self.scope.frames.wip_frame().bump;
        Some(lazy_nodes.into_vnode(NodeFactory { bump }))
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
        (self.scope.shared.submit_task)(task)
    }

    /// This hook enables the ability to expose state to children further down the VirtualDOM Tree.
    ///
    /// This is a hook, so it should not be called conditionally!
    ///
    /// The init method is ran *only* on first use, otherwise it is ignored. However, it uses hooks (ie `use`)
    /// so don't put it in a conditional.
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
    ///     cx.provide_state(SharedState("world"));
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
        self.scope
            .shared_contexts
            .borrow_mut()
            .insert(TypeId::of::<T>(), Rc::new(value))
            .map(|f| f.downcast::<T>().ok())
            .flatten();
    }

    /// Try to retrive a SharedState with type T from the any parent Scope.
    pub fn consume_state<T: 'static>(self) -> Option<Rc<T>> {
        let getter = &self.scope.shared.get_shared_context;
        let ty = TypeId::of::<T>();
        let idx = self.scope.our_arena_idx;
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
        self.scope.new_subtree()
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
        self.scope.subtree()
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
    pub fn use_hook<State, Output, Init, Run, Cleanup>(
        self,
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
        if self.scope.hooks.at_end() {
            self.scope.hooks.push_hook(
                initializer(self.scope.hooks.len()),
                Box::new(|raw| {
                    //
                    let s = raw.downcast::<State>().unwrap();
                    cleanup(s);
                }),
            );
        }

        runner(self.scope.hooks.next::<State>().expect(HOOK_ERR_MSG))
    }
}

const HOOK_ERR_MSG: &str = r###"
Unable to retrive the hook that was initialized in this index.
Consult the `rules of hooks` to understand how to use hooks properly.

You likely used the hook in a conditional. Hooks rely on consistent ordering between renders.
Any function prefixed with "use" should not be called conditionally.
"###;
