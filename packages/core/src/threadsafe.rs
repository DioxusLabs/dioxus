//! A threadsafe wrapper for the VirtualDom

use std::sync::{Arc, Mutex, MutexGuard};

use crate::VirtualDom;

/// A threadsafe wrapper for the Dioxus VirtualDom.
///
/// The Dioxus VirtualDom is not normally `Send` because user code can contain non-`Send` types. However, it is important
/// to have a VirtualDom that is `Send` when used in server-side code since very few web frameworks support non-send
/// handlers.
///
/// To address this, we have the `ThreadsafeVirtualDom` type which is a threadsafe wrapper for the VirtualDom. To access
/// the VirtualDom, it must be first unlocked using the `lock` method. This locks the VirtualDom through a mutex and
/// prevents any user code from leaking out. It is not possible to acquire any non-`Send` types from inside the VirtualDom.
///
/// The only way data may be accessed through the VirtualDom is from the "root props" method or by accessing a `Scope`
/// directly. Even then, it's not possible to access any hook data. This means that non-Send types are only "in play"
/// while the VirtualDom is locked with a non-Send marker.
///
/// Note that calling "wait for work" on the regular VirtualDom is inherently non-Send. If there are async tasks that
/// need to be awaited, they must be done thread-local since we don't place any requirements on user tasks. This can be
/// done with the function "spawn_local" in either tokio or async_std.
pub struct ThreadsafeVirtualDom {
    inner: Arc<Mutex<VirtualDom>>,
}

impl ThreadsafeVirtualDom {
    pub fn new(inner: VirtualDom) -> Self {
        let inner = Arc::new(Mutex::new(inner));
        Self { inner }
    }

    pub fn lock(&self) -> Option<VirtualDomGuard> {
        let locked = self.inner.lock().unwrap();
        Some(VirtualDomGuard { guard: locked })
    }
}

unsafe impl Send for ThreadsafeVirtualDom {}

pub struct VirtualDomGuard<'a> {
    guard: MutexGuard<'a, VirtualDom>,
}

impl<'a> std::ops::Deref for VirtualDomGuard<'a> {
    type Target = MutexGuard<'a, VirtualDom>;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a> std::ops::DerefMut for VirtualDomGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}
