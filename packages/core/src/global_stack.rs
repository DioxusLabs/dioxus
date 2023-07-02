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

use std::sync::atomic::AtomicUsize;

use crate::ScopeId;

pub static CURRENT_SCOPE: AtomicUsize = AtomicUsize::new(0);

/// The current scope id.
///
/// This is specific to a virtualdom, so it might not be unique between two calls
pub fn current_scope() -> ScopeId {
    let id = CURRENT_SCOPE.load(std::sync::atomic::Ordering::Relaxed);
    ScopeId(id)
}

pub(crate) fn set_current_scope(id: ScopeId) {
    CURRENT_SCOPE.store(id.0, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn clear_current_scope(id: ScopeId) {
    CURRENT_SCOPE.store(0, std::sync::atomic::Ordering::Relaxed);
}
