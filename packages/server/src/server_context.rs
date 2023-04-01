use std::sync::{Arc, PoisonError, RwLock, RwLockWriteGuard};

use anymap::{any::Any, Map};

type SendSyncAnyMap = Map<dyn Any + Send + Sync + 'static>;

/// A shared context for server functions. This allows you to pass data between your server and the server functions like authentication session data.
#[derive(Clone)]
pub struct DioxusServerContext {
    shared_context: Arc<RwLock<SendSyncAnyMap>>,
}

impl Default for DioxusServerContext {
    fn default() -> Self {
        Self {
            shared_context: Arc::new(RwLock::new(SendSyncAnyMap::new())),
        }
    }
}

impl DioxusServerContext {
    pub fn get<T: Any + Send + Sync + Clone + 'static>(&self) -> Option<T> {
        self.shared_context.read().ok()?.get::<T>().cloned()
    }

    pub fn insert<T: Any + Send + Sync + 'static>(
        &mut self,
        value: T,
    ) -> Result<(), PoisonError<RwLockWriteGuard<'_, SendSyncAnyMap>>> {
        self.shared_context
            .write()
            .map(|mut map| map.insert(value))
            .map(|_| ())
    }
}
