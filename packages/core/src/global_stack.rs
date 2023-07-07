//! A globally-accessible metadata interface for VirtualDoms.
//!
//! Whenever a VirtualDom is executing, it will perform some logging and metadata tracking.
//!
//! This module exposes that metadata so that it can be used by other libraries without needing to
//! explicitly track it themselves.
//!
//! For instance, libraries like Fermi take advantage of this to allow automatic state subscriptions using the Atom type.
//!
//! The ScopeID is set when:
//! - An async task is polled
//! - An event handler is called
//! - A component is rendered

use std::{cell::Cell, sync::atomic::AtomicU64};

use slab::Slab;

use crate::ScopeId;

/// All the virtualdoms register themselves with this global handle
///
/// From here, we can inject events to any virtualdom
///
/// We can't do things that would require a reference to the dom, but that's okay
pub struct GlobalHandle {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualDomId(u64);

pub static GLOBAL: GlobalHandle = GlobalHandle {};

thread_local! {
    static CURRENT_VIRTUALDOM: Cell<Option<VirtualDomId>> = Cell::new(None);
}

impl GlobalHandle {
    pub fn next(&self) -> VirtualDomId {
        todo!()
    }

    // uses a thread-local to get the currently-running virtualdom on the caller's thread
    pub fn current_virtualdom() -> Option<VirtualDomId> {
        CURRENT_VIRTUALDOM.with(|f| f.get())
    }

    // Spawn a future on the current virtualdom
    pub fn spawn(&self) {
        todo!()
    }
}
