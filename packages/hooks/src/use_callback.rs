use dioxus_core::ScopeState;
use std::future::Future;

#[macro_export]
macro_rules! use_callback {
    // ($cx:ident, || || $($rest:tt)*) => { use_callback( $cx, (), |_| $($rest)* ) };
    // ($cx:ident, || || $($rest:tt)*) => { use_callback( $cx, (), |_| $($rest)* ) };
    ($cx:ident, || $($rest:tt)*) => {
        use_callback(
            $cx,
            move || $($rest)*
        )
    };
    ($cx:ident, |$($args:tt),* | $($rest:tt)*) => {
        use_callback(
            $cx,
            move || $($rest)*
        )
    };
    ($cx:ident, $($rest:tt)*) => {
        use_callback(
            $cx,
            move || $($rest)*
        )
    };
}

pub fn use_callback<T, R, F>(cx: &ScopeState, make: impl FnOnce() -> R) -> impl FnMut(T) + '_
where
    R: FnMut(T) -> F + 'static,
    F: Future<Output = ()> + 'static,
{
    let mut hook = make();

    move |evt| cx.spawn(hook(evt))
}

fn _it_works(cx: &ScopeState) {
    let _p = use_callback(cx, || {
        |()| async {
            //
        }
    });

    // let p = use_callback!(cx, || |evt| async {
    //     //
    // });
}
