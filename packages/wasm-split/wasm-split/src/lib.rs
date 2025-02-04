use std::{
    cell::Cell,
    ffi::c_void,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

pub use wasm_split_macro::wasm_split;

pub type LoadCallbackFn = unsafe extern "C" fn(*const c_void, bool) -> ();
pub type LoadFn = unsafe extern "C" fn(LoadCallbackFn, *const c_void) -> ();

type Lazy = async_once_cell::Lazy<Option<()>, SplitLoaderFuture>;

pub struct LazySplitLoader {
    lazy: Pin<Rc<Lazy>>,
}

impl LazySplitLoader {
    pub unsafe fn new(load: LoadFn) -> Self {
        Self {
            lazy: Rc::pin(Lazy::new(SplitLoaderFuture::new(SplitLoader::new(load)))),
        }
    }
}

pub async fn ensure_loaded(loader: &'static std::thread::LocalKey<LazySplitLoader>) -> Option<()> {
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
    Completed(Option<()>),
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
        web_sys::console::log_1(&"complete fired".into());
        self.state.set(SplitLoaderState::Completed(if success {
            Some(())
        } else {
            None
        }));
        match self.waker.take() {
            Some(waker) => {
                waker.wake();
            }
            _ => {}
        }
    }
}

struct SplitLoaderFuture {
    loader: Rc<SplitLoader>,
}

impl SplitLoaderFuture {
    fn new(loader: Rc<SplitLoader>) -> Self {
        SplitLoaderFuture { loader }
    }
}

impl Future for SplitLoaderFuture {
    type Output = Option<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<()>> {
        web_sys::console::log_1(&"polling".into());
        web_sys::console::log_1(&format!("{:?}", self.loader.state.get()).into());

        match self.loader.state.get() {
            SplitLoaderState::Deferred(load) => {
                self.loader.state.set(SplitLoaderState::Pending);
                self.loader.waker.set(Some(cx.waker().clone()));
                web_sys::console::log_1(&"calling load".into());
                unsafe {
                    load(
                        load_callback,
                        Rc::<SplitLoader>::into_raw(self.loader.clone()) as *const c_void,
                    )
                };
                Poll::Pending
            }
            SplitLoaderState::Pending => {
                web_sys::console::log_1(&"calling pending".into());
                self.loader.waker.set(Some(cx.waker().clone()));
                Poll::Pending
            }
            SplitLoaderState::Completed(value) => {
                web_sys::console::log_1(&"calling complete".into());

                Poll::Ready(value)
            }
        }
    }
}

unsafe extern "C" fn load_callback(loader: *const c_void, success: bool) {
    unsafe { Rc::from_raw(loader as *const SplitLoader) }.complete(success);
}
