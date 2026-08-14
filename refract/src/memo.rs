//! Lazy, equality-gated derived values that are readable through lenses.

use std::any::Any;
use std::cell::{Ref, RefCell};
use std::marker::PhantomData;

use crate::runtime::{NodeId, Path, UpdateResult, with_rt};
use crate::store::Readable;

/// A cached derived value.
///
/// Memos are lazy: marking one dirty costs a bit-flip, and recomputation
/// happens on the next read — strictly *before* the read guard is created, so
/// a recomputation (which needs `&mut` to the cache cell) can never
/// invalidate a live `&T` handed out by a guard.
///
/// Memos are equality-gated: subscribers only rerun when the recomputed value
/// actually differs (`PartialEq`), and the Clean/Check/Dirty algorithm makes
/// that cutoff sound even through chains of memos.
///
/// A memo is a [`Readable`], so lenses compose over it: `memo.select(...)`
/// reads a field of the cached value zero-copy. (It is not [`crate::Writable`] —
/// derived state has no meaningful direct write.)
pub struct Memo<T: 'static> {
    id: NodeId,
    _marker: PhantomData<fn() -> T>,
}

impl<T: 'static> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for Memo<T> {}

impl<T: PartialEq + 'static> Memo<T> {
    pub fn new(mut compute: impl FnMut() -> T + 'static) -> Self {
        let id = with_rt(|rt| {
            rt.create_node(
                Some(Box::new(RefCell::new(None::<T>))),
                None, // patched below once we know the id
                false,
            )
        });
        let update = move || {
            let new = compute();
            let mut cell = with_rt(|rt| rt.borrow_value_mut(id));
            let slot = cell
                .downcast_mut::<Option<T>>()
                .expect("memo type mismatch");
            match slot {
                Some(old) if *old == new => UpdateResult::Unchanged,
                _ => {
                    *slot = Some(new);
                    UpdateResult::Changed
                }
            }
        };
        with_rt(|rt| rt.set_update(id, Box::new(update)));
        Memo {
            id,
            _marker: PhantomData,
        }
    }
}

impl<T: 'static> Readable for Memo<T> {
    type Target = T;

    fn node(&self) -> NodeId {
        self.id
    }

    fn push_path(&self, _out: &mut Path) {}

    fn project(&self, any: Ref<'static, dyn Any>) -> Ref<'static, T> {
        Ref::map(any, |any| {
            any.downcast_ref::<Option<T>>()
                .expect("memo type mismatch")
                .as_ref()
                .expect("memo read before first computation")
        })
    }
}

/// Convenience: `memo(f)` is `Memo::new(f)`.
pub fn memo<T: PartialEq + 'static>(compute: impl FnMut() -> T + 'static) -> Memo<T> {
    Memo::new(compute)
}
