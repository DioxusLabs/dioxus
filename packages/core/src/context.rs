use crate::innerlude::*;

use futures_util::FutureExt;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    future::Future,
    ops::Deref,
    rc::Rc,
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
    pub fn children(&self) -> ScopeChildren<'src> {
        self.scope.child_nodes()
    }

    /// Create a subscription that schedules a future render for the reference component
    ///
    /// ## Notice: you should prefer using prepare_update and get_scope_id
    ///
    pub fn schedule_update(&self) -> Rc<dyn Fn() + 'static> {
        let cb = self.scope.vdom.schedule_update();
        let id = self.get_scope_id();
        Rc::new(move || cb(id))
    }

    pub fn prepare_update(&self) -> Rc<dyn Fn(ScopeId)> {
        self.scope.vdom.schedule_update()
    }

    pub fn schedule_effect(&self) -> Rc<dyn Fn() + 'static> {
        todo!()
    }

    pub fn schedule_layout_effect(&self) {
        todo!()
    }

    /// Get's this context's ScopeId
    pub fn get_scope_id(&self) -> ScopeId {
        self.scope.our_arena_idx.clone()
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
        let scope_ref = self.scope;
        // let listener_id = &scope_ref.listener_idx;
        Some(lazy_nodes.into_vnode(NodeFactory {
            scope: scope_ref,
            // listener_id,
        }))
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
        self.scope.vdom.submit_task(task)
    }

    /// Add a state globally accessible to child components via tree walking
    pub fn add_shared_state<T: 'static>(self, val: T) -> Option<Rc<dyn Any>> {
        self.scope
            .shared_contexts
            .borrow_mut()
            .insert(TypeId::of::<T>(), Rc::new(val))
    }

    /// Walk the tree to find a shared state with the TypeId of the generic type
    ///
    pub fn consume_shared_state<T: 'static>(self) -> Option<Rc<T>> {
        let mut scope = Some(self.scope);
        let mut par = None;

        let ty = TypeId::of::<T>();
        while let Some(inner) = scope {
            log::debug!(
                "Searching {:#?} for valid shared_context",
                inner.our_arena_idx
            );
            let shared_ctx = {
                let shared_contexts = inner.shared_contexts.borrow();

                log::debug!(
                    "This component has {} shared contexts",
                    shared_contexts.len()
                );
                shared_contexts.get(&ty).map(|f| f.clone())
            };

            if let Some(shared_cx) = shared_ctx {
                log::debug!("found matching cx");
                let rc = shared_cx
                    .clone()
                    .downcast::<T>()
                    .expect("Should not fail, already validated the type from the hashmap");

                par = Some(rc);
                break;
            } else {
                match inner.parent_idx {
                    Some(parent_id) => {
                        scope = unsafe { inner.vdom.get_scope(parent_id) };
                    }
                    None => break,
                }
            }
        }
        par
    }

    /// Store a value between renders
    ///
    /// This is *the* foundational hook for all other hooks.
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
}
