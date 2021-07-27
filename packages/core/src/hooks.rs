//! Built-in hooks
//!
//! This module contains all the low-level built-in hooks that require 1st party support to work.
//!
//! Hooks:
//! - use_hook
//! - use_state_provider
//! - use_state_consumer
//! - use_task
//! - use_suspense

use crate::innerlude::*;
use futures_util::FutureExt;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    future::Future,
    rc::Rc,
};

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
pub fn use_provide_state<'src, Pr, T, F>(cx: Context<'src, Pr>, init: F) -> &'src Rc<T>
where
    T: 'static,
    F: FnOnce() -> T,
{
    let ty = TypeId::of::<T>();
    let contains_key = cx.scope.shared_contexts.borrow().contains_key(&ty);

    let is_initialized = cx.use_hook(
        |_| false,
        |s| {
            let i = s.clone();
            *s = true;
            i
        },
        |_| {},
    );

    match (is_initialized, contains_key) {
        // Do nothing, already initialized and already exists
        (true, true) => {}

        // Needs to be initialized
        (false, false) => {
            log::debug!("Initializing context...");
            cx.add_shared_state(init());
            log::info!(
                "There are now {} shared contexts for scope {:?}",
                cx.scope.shared_contexts.borrow().len(),
                cx.scope.our_arena_idx,
            );
        }

        _ => debug_assert!(false, "Cannot initialize two contexts of the same type"),
    };

    use_consume_state::<T, _>(cx)
}

/// There are hooks going on here!
pub fn use_consume_state<'src, T: 'static, P>(cx: Context<'src, P>) -> &'src Rc<T> {
    use_try_consume_state::<T, _>(cx).unwrap()
}

/// Uses a context, storing the cached value around
///
/// If a context is not found on the first search, then this call will be  "dud", always returning "None" even if a
/// context was added later. This allows using another hook as a fallback
///
pub fn use_try_consume_state<'src, T: 'static, P>(cx: Context<'src, P>) -> Option<&'src Rc<T>> {
    struct UseContextHook<C>(Option<Rc<C>>);

    cx.use_hook(
        move |_| UseContextHook(cx.consume_shared_state::<T>()),
        move |hook| hook.0.as_ref(),
        |_| {},
    )
}

/// Awaits the given task, forcing the component to re-render when the value is ready.
///
///
///
///
pub fn use_task<'src, Out, Fut, Init, P>(
    cx: Context<'src, P>,
    task_initializer: Init,
) -> (&'src TaskHandle, &'src Option<Out>)
where
    Out: 'static,
    Fut: Future<Output = Out> + 'static,
    Init: FnOnce() -> Fut + 'src,
{
    struct TaskHook<T> {
        handle: TaskHandle,
        task_dump: Rc<RefCell<Option<T>>>,
        value: Option<T>,
    }

    // whenever the task is complete, save it into th
    cx.use_hook(
        move |hook_idx| {
            let task_fut = task_initializer();

            let task_dump = Rc::new(RefCell::new(None));

            let slot = task_dump.clone();

            let updater = cx.prepare_update();
            let update_id = cx.get_scope_id();

            let originator = cx.scope.our_arena_idx.clone();

            let handle = cx.submit_task(Box::pin(task_fut.then(move |output| async move {
                *slot.as_ref().borrow_mut() = Some(output);
                updater(update_id);
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
                handle,
            }
        },
        |hook| {
            if let Some(val) = hook.task_dump.as_ref().borrow_mut().take() {
                hook.value = Some(val);
            }
            (&hook.handle, &hook.value)
        },
        |_| {},
    )
}

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
pub fn use_suspense<'src, Out, Fut, Cb, P>(
    cx: Context<'src, P>,
    task_initializer: impl FnOnce() -> Fut,
    user_callback: Cb,
) -> DomTree<'src>
where
    Fut: Future<Output = Out> + 'static,
    Out: 'static,
    Cb: for<'a> Fn(SuspendedContext<'a>, &Out) -> DomTree<'a> + 'static,
{
    cx.use_hook(
        move |hook_idx| {
            let value = Rc::new(RefCell::new(None));

            let dom_node_id = Rc::new(empty_cell());
            let domnode = dom_node_id.clone();

            let slot = value.clone();

            let callback: SuspendedCallback = Box::new(move |ctx: SuspendedContext| {
                let v: std::cell::Ref<Option<Box<dyn Any>>> = slot.as_ref().borrow();
                match v.as_ref() {
                    Some(a) => {
                        let v: &dyn Any = a.as_ref();
                        let real_val = v.downcast_ref::<Out>().unwrap();
                        user_callback(ctx, real_val)
                    }
                    None => {
                        //
                        Some(VNode {
                            dom_id: empty_cell(),
                            key: None,
                            kind: VNodeKind::Suspended {
                                node: domnode.clone(),
                            },
                        })
                    }
                }
            });

            let originator = cx.scope.our_arena_idx.clone();
            let task_fut = task_initializer();
            let domnode = dom_node_id.clone();

            let slot = value.clone();
            cx.submit_task(Box::pin(task_fut.then(move |output| async move {
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
                scope: &cx.scope,
                props: &(),
            };
            let csx = SuspendedContext { inner: cx };
            (&hook.callback)(csx)
        },
        |_| {},
    )
}

pub(crate) struct SuspenseHook {
    pub value: Rc<RefCell<Option<Box<dyn Any>>>>,
    pub callback: SuspendedCallback,
    pub dom_node_id: Rc<Cell<Option<ElementId>>>,
}
type SuspendedCallback = Box<dyn for<'a> Fn(SuspendedContext<'a>) -> DomTree<'a>>;
pub struct SuspendedContext<'a> {
    pub(crate) inner: Context<'a, ()>,
}
impl<'src> SuspendedContext<'src> {
    pub fn render<F: FnOnce(NodeFactory<'src>) -> VNode<'src>>(
        self,
        lazy_nodes: LazyNodes<'src, F>,
    ) -> DomTree<'src> {
        let scope_ref = self.inner.scope;

        Some(lazy_nodes.into_vnode(NodeFactory { scope: scope_ref }))
    }
}
