#[macro_export]
/// A helper macro for using hooks and properties in async environements.
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

mod use_on_unmount;
pub use use_on_unmount::*;

mod usecontext;
pub use usecontext::*;

mod usestate;
pub use usestate::{use_state, UseState};

mod useref;
pub use useref::*;

mod use_shared_state;
pub use use_shared_state::*;

mod usecoroutine;
pub use usecoroutine::*;

mod usefuture;
pub use usefuture::*;

mod useeffect;
pub use useeffect::*;

mod usecallback;
pub use usecallback::*;

mod usememo;
pub use usememo::*;

mod userootcontext;
pub use userootcontext::*;
