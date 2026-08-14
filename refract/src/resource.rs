//! Async derived values: "memos over futures".

use std::any::Any;
use std::cell::{Ref, RefCell};
use std::future::Future;
use std::marker::PhantomData;

use crate::runtime::{NodeId, Path, UpdateResult, with_rt};
use crate::store::{ReadGuard, Readable};

/// The lifecycle of a resource value.
///
/// `Reloading` keeps the previous value (moved, not cloned) while a restarted
/// future is in flight, so UIs can keep rendering stale data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceState<T> {
    Pending,
    Ready(T),
    Reloading(T),
}

impl<T> ResourceState<T> {
    /// The current value, if any (stale values during reload included).
    pub fn value(&self) -> Option<&T> {
        match self {
            ResourceState::Pending => None,
            ResourceState::Ready(value) | ResourceState::Reloading(value) => Some(value),
        }
    }

    pub fn is_loading(&self) -> bool {
        !matches!(self, ResourceState::Ready(_))
    }
}

/// An async derived value.
///
/// The `source` closure runs synchronously under tracking: every store/memo
/// read made *while constructing the future* is a dependency. Reads after the
/// first `await` are intentionally untracked — tracking across `await` with a
/// thread-local observer would misattribute dependencies from interleaved
/// tasks.
///
/// When a dependency changes, the in-flight future is dropped (cancellation
/// is `Drop` in Rust) and a fresh one is created; a `Ready` value degrades to
/// `Reloading` rather than being thrown away.
///
/// The driver future is stored as `Pin<Box<dyn Future>>` in a slot *separate*
/// from the value cell: the future is self-referential after its first poll
/// (hence the pinned box), and polling needs `&mut` to the future at the same
/// time as effects may be reading the resource state through lenses — keeping
/// them in separate cells is what makes that borrow-safe.
pub struct Resource<T: 'static> {
    id: NodeId,
    _marker: PhantomData<fn() -> T>,
}

impl<T: 'static> Clone for Resource<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for Resource<T> {}

impl<T: 'static> Resource<T> {
    pub fn new<F>(mut source: impl FnMut() -> F + 'static) -> Self
    where
        F: Future<Output = T> + 'static,
    {
        let id = with_rt(|rt| {
            rt.create_node(
                Some(Box::new(RefCell::new(ResourceState::<T>::Pending))),
                None,
                true, // eager: restarts are scheduled like effects
            )
        });
        let update = move || {
            // Degrade Ready -> Reloading, keeping the old value by move.
            {
                let mut cell = with_rt(|rt| rt.borrow_value_mut(id));
                let state = cell
                    .downcast_mut::<ResourceState<T>>()
                    .expect("resource type mismatch");
                if let ResourceState::Ready(value) =
                    std::mem::replace(state, ResourceState::Pending)
                {
                    *state = ResourceState::Reloading(value);
                }
            }
            // Tracked: runs under this node as observer.
            let future = source();
            let driver = Box::pin(async move {
                let value = future.await;
                with_rt(|rt| {
                    {
                        let mut cell = rt.borrow_value_mut(id);
                        let state = cell
                            .downcast_mut::<ResourceState<T>>()
                            .expect("resource type mismatch");
                        *state = ResourceState::Ready(value);
                    }
                    rt.notify_write(id, &[]);
                });
            });
            with_rt(|rt| rt.set_driver(id, driver));
            // The observable state changed (Ready -> Reloading or a fresh
            // Pending): let subscribers see the loading state.
            UpdateResult::Changed
        };
        with_rt(|rt| {
            rt.set_update(id, Box::new(update));
            rt.ensure(id);
        });
        Resource {
            id,
            _marker: PhantomData,
        }
    }

    /// A tracked guard to the current value, if one exists. Zero-copy: the
    /// guard projects into `Ready`/`Reloading` in place.
    pub fn try_read(&self) -> Option<ReadGuard<T>> {
        ReadGuard::filter_map(self.read(), ResourceState::value)
    }
}

impl<T: 'static> Readable for Resource<T> {
    type Target = ResourceState<T>;

    fn node(&self) -> NodeId {
        self.id
    }

    fn push_path(&self, _out: &mut Path) {}

    fn project(&self, any: Ref<'static, dyn Any>) -> Ref<'static, ResourceState<T>> {
        Ref::map(any, |any| {
            any.downcast_ref::<ResourceState<T>>()
                .expect("resource type mismatch")
        })
    }
}

/// Convenience: `resource(f)` is `Resource::new(f)`.
pub fn resource<T: 'static, F>(source: impl FnMut() -> F + 'static) -> Resource<T>
where
    F: Future<Output = T> + 'static,
{
    Resource::new(source)
}
