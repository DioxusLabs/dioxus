use std::{
    cell::{RefCell, UnsafeCell},
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ops::DerefMut,
    pin::Pin,
    process::Output,
    rc::Rc,
    sync::Arc,
};

use futures_task::{waker, RawWaker, RawWakerVTable, Waker};

pub trait RcWake {
    fn wake(self: Rc<Self>) {
        Self::wake_by_ref(&self)
    }
    fn wake_by_ref(arc_self: &Rc<Self>);
}

pub fn make_rc_waker<T: RcWake>(rc: Rc<T>) -> Waker {
    unsafe { Waker::from_raw(RawWaker::new(Rc::into_raw(rc).cast(), rc_vtable::<T>())) }
}

fn rc_vtable<T: RcWake>() -> &'static RawWakerVTable {
    &RawWakerVTable::new(
        clone_rc_raw::<T>,
        wake_rc_raw::<T>,
        wake_by_ref_rc_raw::<T>,
        drop_rc_raw::<T>,
    )
}

// FIXME: panics on Rc::clone / refcount changes could wreak havoc on the
// code here. We should guard against this by aborting.
unsafe fn clone_rc_raw<T: RcWake>(data: *const ()) -> RawWaker {
    let arc = mem::ManuallyDrop::new(Rc::<T>::from_raw(data.cast::<T>()));
    let _rc_clone: mem::ManuallyDrop<_> = arc.clone();
    RawWaker::new(data, rc_vtable::<T>())
}

unsafe fn wake_rc_raw<T: RcWake>(data: *const ()) {
    let arc: Rc<T> = Rc::from_raw(data.cast::<T>());
    arc.wake();
}

unsafe fn wake_by_ref_rc_raw<T: RcWake>(data: *const ()) {
    let arc = mem::ManuallyDrop::new(Rc::<T>::from_raw(data.cast::<T>()));
    arc.wake();
}

unsafe fn drop_rc_raw<T: RcWake>(data: *const ()) {
    drop(Rc::<T>::from_raw(data.cast::<T>()))
}
