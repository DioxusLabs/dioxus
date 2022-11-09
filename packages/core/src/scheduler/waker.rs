use std::task::{RawWaker, RawWakerVTable, Waker};
use std::{mem, rc::Rc};

pub trait RcWake: Sized {
    /// Create a waker from this self-wakening object
    fn waker(self: &Rc<Self>) -> Waker {
        unsafe fn rc_vtable<T: RcWake>() -> &'static RawWakerVTable {
            &RawWakerVTable::new(
                |data| {
                    let arc = mem::ManuallyDrop::new(Rc::<T>::from_raw(data.cast::<T>()));
                    let _rc_clone: mem::ManuallyDrop<_> = arc.clone();
                    RawWaker::new(data, rc_vtable::<T>())
                },
                |data| Rc::from_raw(data.cast::<T>()).wake(),
                |data| {
                    let arc = mem::ManuallyDrop::new(Rc::<T>::from_raw(data.cast::<T>()));
                    RcWake::wake_by_ref(&arc);
                },
                |data| drop(Rc::<T>::from_raw(data.cast::<T>())),
            )
        }

        unsafe {
            Waker::from_raw(RawWaker::new(
                Rc::into_raw(self.clone()).cast(),
                rc_vtable::<Self>(),
            ))
        }
    }

    fn wake_by_ref(arc_self: &Rc<Self>);

    fn wake(self: Rc<Self>) {
        Self::wake_by_ref(&self)
    }
}
