use std::rc::Rc;

use dioxus_core::{prelude::EventHandler, ScopeState};
use std::future::Future;

#[macro_export]
macro_rules! use_callback {
    ($cx:ident, || || $($rest:tt)*) => { use_callback( $cx, (), |_| $($rest)* ) };
    ($cx:ident, || |$myarg:ident| $($rest:tt)*) => {
        use_callback(
            $cx,
            || |$myarg| async {}
        )
    };
}
pub fn use_callback<'a, T, R, F>(cx: &'a ScopeState, make: impl FnOnce() -> R) -> impl FnMut(T) + 'a
where
    R: FnMut(T) -> F + 'static,
    F: Future<Output = ()> + 'static,
{
    let mut hook = make();

    move |evt| cx.spawn(hook(evt))
}

fn it_works(cx: &ScopeState) {
    let p = use_callback(cx, || {
        |()| async {
            //
        }
    });

    // let p = use_callback!(cx, || |evt| async {
    //     //
    // });
}
