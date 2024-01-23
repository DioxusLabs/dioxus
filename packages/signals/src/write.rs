use std::ops::DerefMut;

use crate::read::Readable;

pub trait Writable<T: 'static>: Readable<T> {
    type Mut<R: ?Sized + 'static>: DerefMut<Target = R>;

    fn map_mut<I, U: ?Sized + 'static, F: FnOnce(&mut I) -> &mut U>(
        ref_: Self::Mut<I>,
        f: F,
    ) -> Self::Mut<U>;

    fn try_map_mut<I, U: ?Sized + 'static, F: FnOnce(&mut I) -> Option<&mut U>>(
        ref_: Self::Mut<I>,
        f: F,
    ) -> Option<Self::Mut<U>>;

    fn write(&self) -> Self::Mut<T>;

    #[track_caller]
    fn with_mut<O>(&self, f: impl FnOnce(&mut T) -> O) -> O {
        f(&mut *self.write())
    }

    /// Set the value of the signal. This will trigger an update on all subscribers.
    #[track_caller]
    fn set(&mut self, value: T) {
        *self.write() = value;
    }

    /// Invert the boolean value of the signal. This will trigger an update on all subscribers.
    fn toggle(&mut self)
    where
        T: std::ops::Not<Output = T> + Clone,
    {
        self.set(!self.cloned());
    }

    /// Index into the inner value and return a reference to the result.
    #[track_caller]
    fn index_mut<I>(&self, index: I) -> Self::Mut<T::Output>
    where
        T: std::ops::IndexMut<I>,
    {
        Self::map_mut(self.write(), |v| v.index_mut(index))
    }
}
