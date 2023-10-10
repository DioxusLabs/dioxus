#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![cfg_attr(feature = "nightly-features", feature(debug_refcell))]

#[macro_export]
/// A helper macro for using hooks and properties in async environments.
///
/// # Usage
///
///
/// ```
/// # use dioxus::prelude::*;
/// #
/// # #[derive(Props, PartialEq)]
/// # struct Props {
/// #    prop: String,
/// # }
/// # fn Component(cx: Scope<Props>) -> Element {
///
/// let (data) = use_ref(cx, || {});
///
/// let handle_thing = move |_| {
///     to_owned![data, cx.props.prop];
///     cx.spawn(async move {
///         // do stuff
///     });
/// };
/// # handle_thing(());
/// # None }
/// ```
macro_rules! to_owned {
    // Rule matching simple symbols without a path
    ($es:ident $(, $($rest:tt)*)?) => {
        #[allow(unused_mut)]
        let mut $es = $es.to_owned();
        $( to_owned![$($rest)*] )?
    };

    // We need to find the last element in a path, for this we need to unstack the path part by
    // part using, separating what we have with a '@'
    ($($deref:ident).* $(, $($rest:tt)*)?) => {
        to_owned![@ $($deref).* $(, $($rest)*)?]
    };

    // Take the head of the path and add it to the list of $deref
    ($($deref:ident)* @ $head:ident $( . $tail:ident)+ $(, $($rest:tt)*)?) => {
        to_owned![$($deref)* $head @ $($tail).+ $(, $($rest)*)?]
    };
    // We have exhausted the path, use the last as a name
    ($($deref:ident)* @ $last:ident $(, $($rest:tt)*)? ) => {
        #[allow(unused_mut)]
        let mut $last = $($deref .)* $last .to_owned();
        $(to_owned![$($rest)*])?
    };
}

pub mod computed;

mod use_on_destroy;
pub use use_on_destroy::*;

mod use_context;
pub use use_context::*;

mod use_state;
pub use use_state::{use_state, UseState};

mod use_ref;
pub use use_ref::*;

mod use_shared_state;
pub use use_shared_state::*;

mod use_coroutine;
pub use use_coroutine::*;

mod use_future;
pub use use_future::*;

mod use_effect;
pub use use_effect::*;

mod use_callback;
pub use use_callback::*;

mod use_memo;
pub use use_memo::*;

mod use_on_create;
pub use use_on_create::*;
mod use_root_context;
pub use use_root_context::*;
