#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

#[macro_export]
/// A helper macro for using hooks and properties in async environments.
///
/// # Usage
///
///
/// ```
/// # use dioxus::prelude::*;
/// #
/// # #[derive(Props, PartialEq, Clone)]
/// # struct Props {
/// #    prop: String,
/// # }
/// # fn Component(props: Props) -> Element {
///
/// let (data) = use_signal(|| {});
///
/// let handle_thing = move |_| {
///     to_owned![data, props.prop];
///     spawn(async move {
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
mod dependency;
pub use dependency::*;

mod use_callback;
pub use use_callback::*;

mod use_on_destroy;
pub use use_on_destroy::*;

mod use_context;
pub use use_context::*;

mod use_coroutine;
pub use use_coroutine::*;

mod use_future;
pub use use_future::*;

// mod use_sorted;
// pub use use_sorted::*;

mod use_resource;
pub use use_resource::*;

mod use_effect;
pub use use_effect::*;

mod use_memo;
pub use use_memo::*;

// mod use_on_create;
// pub use use_on_create::*;

mod use_root_context;
pub use use_root_context::*;

mod use_hook_did_run;
pub use use_hook_did_run::*;

mod use_signal;
pub use use_signal::*;
