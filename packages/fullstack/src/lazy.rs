use std::{any::Any, pin::Pin, prelude::rust_2024::Future};

pub struct Lazy<T> {
    // init: fn() -> T,
    // instance: std::sync::OnceLock<T>,
    // caller: fn() -> PinnedAny,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + Sync + 'static> Lazy<T> {
    pub const fn lazy() -> Self {
        Self {
            // caller: ,
            _phantom: std::marker::PhantomData,
        }
    }

    // // pub const fn new<F: Future<Output = Result<T, dioxus_core::Error>> + 'static>(
    // //     f: impl FnOnce() -> F + Copy,
    // // ) -> Self {
    // pub const fn new<G: FnOnce() -> F + Copy + 'static, F: Future<Output = T> + 'static>(
    //     f: G,
    // ) -> Self {
    //     // unsafe extern "C" fn __ctor() {}

    //     unsafe extern "C" fn __my_ctor<T>() {}

    //     // static __CTOR: unsafe extern "C" fn() = MY_CTOR as _;

    //     // const SIZE: usize = std::mem::size_of::<G>;

    //     // const my_entry: fn() -> PinnedAny = ;

    //     // const it_works: LazyInner = LazyInner {
    //     //     static_entry: my_entry,
    //     // };

    //     // static ENTRY: LazyInner = ;

    //     Self {
    //         // caller: __lazy_static_entry::<G, F, T>,
    //         _phantom: std::marker::PhantomData,
    //     }
    // }

    pub async fn initialize(&self) {}

    pub fn set(&self, pool: T) -> Result<(), dioxus_core::Error> {
        todo!()
    }
}

fn __lazy_static_entry<T, G, M>() -> PinnedAny
where
    T: 'static + FnOnce() -> G,
    G: Future<Output = M> + 'static,
    M: 'static + Send + Sync,
{
    todo!()
    // __fixed_size_lazy_static_initializer::<T, G, M>()
}

type PinnedAny = Pin<Box<dyn Future<Output = Box<dyn Any + Send + Sync>>>>;

struct LazyInner {
    static_entry: fn() -> PinnedAny,
}

impl<T> std::ops::Deref for Lazy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}
