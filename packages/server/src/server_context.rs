use std::sync::{Arc, PoisonError, RwLock, RwLockWriteGuard};

use anymap::{any::Any, Map};

type SendSyncAnyMap = Map<dyn Any + Send + Sync + 'static>;

/// A shared context for server functions.
/// This allows you to pass data between your server framework and the server functions. This can be used to pass request information or information about the state of the server. For example, you could pass authentication data though this context to your server functions.
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
    /// Clone a value from the shared server context
    pub fn get<T: Any + Send + Sync + Clone + 'static>(&self) -> Option<T> {
        self.shared_context.read().ok()?.get::<T>().cloned()
    }

    /// Insert a value into the shared server context
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

/// Generate a server context from a tuple of values
macro_rules! server_context {
    ($({$(($name:ident: $ty:ident)),*}),*) => {
        $(
            #[allow(unused_mut)]
            impl< $($ty: Send + Sync + 'static),* > From<($($ty,)*)> for $crate::server_context::DioxusServerContext {
                fn from(( $($name,)* ): ($($ty,)*)) -> Self {
                    let mut context = $crate::server_context::DioxusServerContext::default();
                    $(context.insert::<$ty>($name).unwrap();)*
                    context
                }
            }
        )*
    };
}

server_context!(
    {},
    {(a: A)},
    {(a: A), (b: B)},
    {(a: A), (b: B), (c: C)},
    {(a: A), (b: B), (c: C), (d: D)},
    {(a: A), (b: B), (c: C), (d: D), (e: E)},
    {(a: A), (b: B), (c: C), (d: D), (e: E), (f: F)},
    {(a: A), (b: B), (c: C), (d: D), (e: E), (f: F), (g: G)},
    {(a: A), (b: B), (c: C), (d: D), (e: E), (f: F), (g: G), (h: H)},
    {(a: A), (b: B), (c: C), (d: D), (e: E), (f: F), (g: G), (h: H), (i: I)},
    {(a: A), (b: B), (c: C), (d: D), (e: E), (f: F), (g: G), (h: H), (i: I), (j: J)},
    {(a: A), (b: B), (c: C), (d: D), (e: E), (f: F), (g: G), (h: H), (i: I), (j: J), (k: K)},
    {(a: A), (b: B), (c: C), (d: D), (e: E), (f: F), (g: G), (h: H), (i: I), (j: J), (k: K), (l: L)},
    {(a: A), (b: B), (c: C), (d: D), (e: E), (f: F), (g: G), (h: H), (i: I), (j: J), (k: K), (l: L), (m: M)},
    {(a: A), (b: B), (c: C), (d: D), (e: E), (f: F), (g: G), (h: H), (i: I), (j: J), (k: K), (l: L), (m: M), (n: N)}
);
