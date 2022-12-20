use std::mem;
use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable, Waker};

pub trait ArcWake: Sized {
    /// Create a waker from this self-wakening object
    fn waker(self: &Arc<Self>) -> Waker {
        unsafe fn rc_vtable<T: ArcWake>() -> &'static RawWakerVTable {
            &RawWakerVTable::new(
                |data| {
                    let arc = mem::ManuallyDrop::new(Arc::<T>::from_raw(data.cast::<T>()));
                    let _rc_clone: mem::ManuallyDrop<_> = arc.clone();
                    RawWaker::new(data, rc_vtable::<T>())
                },
                |data| Arc::from_raw(data.cast::<T>()).wake(),
                |data| {
                    let arc = mem::ManuallyDrop::new(Arc::<T>::from_raw(data.cast::<T>()));
                    ArcWake::wake_by_ref(&arc);
                },
                |data| drop(Arc::<T>::from_raw(data.cast::<T>())),
            )
        }

        unsafe {
            Waker::from_raw(RawWaker::new(
                Arc::into_raw(self.clone()).cast(),
                rc_vtable::<Self>(),
            ))
        }
    }

    fn wake_by_ref(arc_self: &Arc<Self>);

    fn wake(self: Arc<Self>) {
        Self::wake_by_ref(&self)
    }
}
