use std::{
    cell::Cell,
    ffi::c_void,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::atomic::AtomicBool,
    task::{Context, Poll, Waker},
    thread::LocalKey,
};

pub use wasm_split_macro::{lazy_loader, wasm_split};

pub type LoadCallbackFn = unsafe extern "C" fn(*const c_void, bool) -> ();
pub type LoadFn = unsafe extern "C" fn(LoadCallbackFn, *const c_void) -> ();

type Lazy = async_once_cell::Lazy<bool, SplitLoaderFuture>;

pub struct LazySplitLoader {
    lazy: Pin<Rc<Lazy>>,
}

impl LazySplitLoader {
    pub unsafe fn new(load: LoadFn) -> Self {
        Self {
            lazy: Rc::pin(Lazy::new({
                SplitLoaderFuture {
                    loader: SplitLoader::new(load),
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
}

pub async fn ensure_loaded(loader: &'static std::thread::LocalKey<LazySplitLoader>) -> bool {
    *loader.with(|inner| inner.lazy.clone()).as_ref().await
}

pub struct SplitLoader {
    state: Cell<SplitLoaderState>,
    waker: Cell<Option<Waker>>,
}

#[derive(Clone, Copy, Debug)]
enum SplitLoaderState {
    Deferred(LoadFn),
    Pending,
    Completed(bool),
}

impl SplitLoader {
    /// Create a new split loader
    pub fn new(load: LoadFn) -> Rc<Self> {
        Rc::new(SplitLoader {
            state: Cell::new(SplitLoaderState::Deferred(load)),
            waker: Cell::new(None),
        })
    }

    /// Mark the split loader as complete with the given success value
    pub fn complete(&self, success: bool) {
        self.state.set(SplitLoaderState::Completed(success));
        if let Some(waker) = self.waker.take() {
            waker.wake()
        }
    }
}

struct SplitLoaderFuture {
    loader: Rc<SplitLoader>,
}

impl Future for SplitLoaderFuture {
    type Output = bool;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<bool> {
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

unsafe extern "C" fn load_callback(loader: *const c_void, success: bool) {
    unsafe { Rc::from_raw(loader as *const SplitLoader) }.complete(success);
}

pub struct LazyLoader<Args, Ret> {
    pub imported: unsafe extern "C" fn(arg: Args) -> Ret,
    pub key: &'static LocalKey<LazySplitLoader>,
    pub loaded: AtomicBool,
}

impl<Args, Ret> LazyLoader<Args, Ret> {
    pub async fn load(&'static self) -> bool {
        let res = *self.key.with(|inner| inner.lazy.clone()).as_ref().await;
        self.loaded
            .store(true, std::sync::atomic::Ordering::Relaxed);
        res
    }

    pub fn call(&'static self, args: Args) -> Option<Ret> {
        if !self.loaded.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }

        Some(unsafe { (self.imported)(args) })
    }
}
