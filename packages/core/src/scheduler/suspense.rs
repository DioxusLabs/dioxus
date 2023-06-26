/*
Suspense in Dioxus

Suspense is a feature that allows components to suspend rendering while they are waiting for some data to load.
This allows the application to show some fallback content (such as a loading indicator) while the data is being fetched.

Suspense is easy to opt-in.

To convert a component from a regular component to a suspense component, simply use a hook that calls "suspend" and return None.

This will let Dioxus know to re-run the component when the data is ready.

The `suspend` method will propagate to the nearest suspense boundary, which will then suspend the entire subtree below it.

If there is no suspense boundary above the component, the component will simply not render since it returned `None`.

This means you can slowly adopt suspense in your application without having to change all of your components at once.

And, this means we don't have "colored" components like React does.

When a component wants to progress, it just calls "needs_update" and Dioxus will go and run that component again.

let data = use_future(cx, || fetch_data(props))?;

if !data.is_ready() {
    cx.suspend();
    return None;
}

let data = use_suspense(cx, || fetch_data(props))?;
let data = use_future(cx, || fetch_data(props)).suspend()?;
let data = use_suspense(cx, || fetch_data(props))?;
 */

use futures_util::task::ArcWake;

use super::SchedulerMsg;
use crate::ElementId;
use crate::{innerlude::Mutations, Element, ScopeId};
use std::future::Future;
use std::sync::Arc;
use std::task::Waker;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
};

/// An ID representing an ongoing suspended component
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct SuspenseId(pub usize);

/// A boundary in the VirtualDom that captures all suspended components below it
pub struct SuspenseContext {
    pub(crate) id: ScopeId,
    pub(crate) waiting_on: RefCell<HashSet<SuspenseId>>,
    pub(crate) mutations: RefCell<Mutations<'static>>,
    pub(crate) placeholder: Cell<Option<ElementId>>,
    pub(crate) created_on_stack: Cell<usize>,
}

impl SuspenseContext {
    /// Create a new boundary for suspense
    pub fn new(id: ScopeId) -> Self {
        Self {
            id,
            waiting_on: Default::default(),
            mutations: RefCell::new(Mutations::default()),
            placeholder: Cell::new(None),
            created_on_stack: Cell::new(0),
        }
    }
}

pub struct SuspenseHandle {
    pub(crate) id: SuspenseId,
    pub(crate) tx: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,
}

impl ArcWake for SuspenseHandle {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        _ = arc_self
            .tx
            .unbounded_send(SchedulerMsg::SuspenseNotified(arc_self.id));
    }
}
