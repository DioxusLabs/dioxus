use std::{
    cell::Cell,
    ffi::c_void,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
    thread::LocalKey,
};

pub use wasm_split_macro::{lazy_loader, wasm_split};

pub type Result<T> = std::result::Result<T, SplitLoaderError>;

#[derive(Debug, Clone)]
pub enum SplitLoaderError {
    FailedToLoad,
}

pub struct LazyLoader<Args, Ret> {
    imported: unsafe extern "C" fn(arg: Args) -> Ret,
    key: &'static LocalKey<LazySplitLoader>,
}

impl<Args, Ret> LazyLoader<Args, Ret> {
    pub const unsafe fn new(
        imported: unsafe extern "C" fn(arg: Args) -> Ret,
        key: &'static LocalKey<LazySplitLoader>,
    ) -> Self {
        Self { imported, key }
    }

    pub async fn load(&'static self) -> bool {
        *self.key.with(|inner| inner.lazy.clone()).as_ref().await
    }

    pub fn call(&'static self, args: Args) -> Result<Ret> {
        let Some(true) = self.key.with(|inner| inner.lazy.try_get().copied()) else {
            return Err(SplitLoaderError::FailedToLoad);
        };

        Ok(unsafe { (self.imported)(args) })
    }
}

type Lazy = async_once_cell::Lazy<bool, SplitLoaderFuture>;
type LoadCallbackFn = unsafe extern "C" fn(*const c_void, bool) -> ();
type LoadFn = unsafe extern "C" fn(LoadCallbackFn, *const c_void) -> ();

pub struct LazySplitLoader {
    lazy: Pin<Rc<Lazy>>,
}

impl LazySplitLoader {
    pub unsafe fn new(load: LoadFn) -> Self {
        Self {
            lazy: Rc::pin(Lazy::new({
                SplitLoaderFuture {
                    loader: Rc::new(SplitLoader {
                        state: Cell::new(SplitLoaderState::Deferred(load)),
                        waker: Cell::new(None),
                    }),
                }
            })),
        }
    }

    pub fn is_loaded(&self) -> bool {
        match self.lazy.as_ref().try_get() {
            Some(res) => *res,
            None => false,
        }
    }

    pub async fn ensure_loaded(loader: &'static std::thread::LocalKey<LazySplitLoader>) -> bool {
        *loader.with(|inner| inner.lazy.clone()).as_ref().await
    }
}

struct SplitLoader {
    state: Cell<SplitLoaderState>,
    waker: Cell<Option<Waker>>,
}

#[derive(Clone, Copy, Debug)]
enum SplitLoaderState {
    Deferred(LoadFn),
    Pending,
    Completed(bool),
}

struct SplitLoaderFuture {
    loader: Rc<SplitLoader>,
}

impl Future for SplitLoaderFuture {
    type Output = bool;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<bool> {
        unsafe extern "C" fn load_callback(loader: *const c_void, success: bool) {
            let loader = unsafe { Rc::from_raw(loader as *const SplitLoader) };
            loader.state.set(SplitLoaderState::Completed(success));
            if let Some(waker) = loader.waker.take() {
                waker.wake()
            }
        }

        match self.loader.state.get() {
            SplitLoaderState::Deferred(load) => {
                self.loader.state.set(SplitLoaderState::Pending);
                self.loader.waker.set(Some(cx.waker().clone()));
                unsafe {
                    load(
                        load_callback,
                        Rc::<SplitLoader>::into_raw(self.loader.clone()) as *const c_void,
                    )
                };
                Poll::Pending
            }
            SplitLoaderState::Pending => {
                self.loader.waker.set(Some(cx.waker().clone()));
                Poll::Pending
            }
            SplitLoaderState::Completed(value) => Poll::Ready(value),
        }
    }
}
