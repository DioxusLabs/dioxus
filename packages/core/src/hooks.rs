//! Built-in hooks
//!
//! This module contains all the low-level built-in hooks that require 1st party support to work.
//!
//! Hooks:
//! - [`use_hook`]
//! - [`use_state_provider`]
//! - [`use_state_consumer`]
//! - [`use_task`]
//! - [`use_suspense`]

use crate::innerlude::*;
use futures_util::FutureExt;
use std::{any::Any, cell::RefCell, future::Future, ops::Deref, rc::Rc};

/// Awaits the given task, forcing the component to re-render when the value is ready.
///
/// Returns the handle to the task and the value (if it is ready, else None).
///
/// ```
/// static Example: FC<()> = |cx, props| {
///     let (task, value) = use_task(|| async {
///         timer::sleep(Duration::from_secs(1)).await;
///         "Hello World"
///     });
///
///     match contents {
///         Some(contents) => rsx!(cx, div { "{title}" }),
///         None => rsx!(cx, div { "Loading..." }),
///     }
/// };
/// ```
pub fn use_task<'src, Out, Fut, Init>(
    cx: Context<'src>,
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
        move |_| {
            let task_fut = task_initializer();

            let task_dump = Rc::new(RefCell::new(None));

            let slot = task_dump.clone();

            let updater = cx.schedule_update_any();
            let originator = cx.scope.our_arena_idx;

            let handle = cx.submit_task(Box::pin(task_fut.then(move |output| async move {
                *slot.as_ref().borrow_mut() = Some(output);
                updater(originator);
                originator
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
pub fn use_suspense<'src, Out, Fut, Cb>(
    cx: Context<'src>,
    task_initializer: impl FnOnce() -> Fut,
    user_callback: Cb,
) -> DomTree<'src>
where
    Fut: Future<Output = Out> + 'static,
    Out: 'static,
    Cb: for<'a> Fn(SuspendedContext<'a>, &Out) -> DomTree<'a> + 'static,
{
    /*
    General strategy:
    - Create a slot for the future to dump its output into
    - Create a new future feeding off the user's future that feeds the output into that slot
    - Submit that future as a task
    - Take the task handle id and attach that to our suspended node
    - when the hook runs, check if the value exists
    - if it does, then we can render the node directly
    - if it doesn't, then we render a suspended node along with with the callback and task id
    */
    cx.use_hook(
        move |_| {
            let value = Rc::new(RefCell::new(None));
            let slot = value.clone();
            let originator = cx.scope.our_arena_idx;

            let handle = cx.submit_task(Box::pin(task_initializer().then(
                move |output| async move {
                    *slot.borrow_mut() = Some(Box::new(output) as Box<dyn Any>);
                    originator
                },
            )));

            SuspenseHook { handle, value }
        },
        move |hook| match hook.value.borrow().as_ref() {
            Some(value) => {
                let out = value.downcast_ref::<Out>().unwrap();
                let sus = SuspendedContext {
                    inner: Context { scope: cx.scope },
                };
                user_callback(sus, out)
            }
            None => {
                let value = hook.value.clone();

                cx.render(LazyNodes::new(|f| {
                    let bump = f.bump();

                    use bumpalo::boxed::Box as BumpBox;

                    let f: &mut dyn FnMut(SuspendedContext<'src>) -> DomTree<'src> =
                        bump.alloc(move |sus| {
                            let val = value.borrow();

                            let out = val
                                .as_ref()
                                .unwrap()
                                .as_ref()
                                .downcast_ref::<Out>()
                                .unwrap();

                            user_callback(sus, out)
                        });
                    let callback = unsafe { BumpBox::from_raw(f) };

                    VNode::Suspended(bump.alloc(VSuspended {
                        dom_id: empty_cell(),
                        task_id: hook.handle.our_id,
                        callback: RefCell::new(Some(callback)),
                    }))
                }))
            }
        },
        |_| {},
    )
}

pub(crate) struct SuspenseHook {
    pub handle: TaskHandle,
    pub value: Rc<RefCell<Option<Box<dyn Any>>>>,
}

pub struct SuspendedContext<'a> {
    pub(crate) inner: Context<'a>,
}

impl<'src> SuspendedContext<'src> {
    pub fn render<F: FnOnce(NodeFactory<'src>) -> VNode<'src>>(
        self,
        lazy_nodes: LazyNodes<'src, F>,
    ) -> DomTree<'src> {
        let bump = &self.inner.scope.frames.wip_frame().bump;
        Some(lazy_nodes.into_vnode(NodeFactory { bump }))
    }
}

#[derive(Clone, Copy)]
pub struct NodeRef<'src, T: 'static>(&'src RefCell<Option<T>>);

impl<'a, T> Deref for NodeRef<'a, T> {
    type Target = RefCell<Option<T>>;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

pub fn use_node_ref<T, P>(cx: Context) -> NodeRef<T> {
    cx.use_hook(|_| RefCell::new(None), |f| NodeRef { 0: f }, |_| {})
}
