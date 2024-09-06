use std::iter::FlatMap;
use std::slice::Iter;
use std::{fmt::Display, str::FromStr};

/// Something that can be created from an entire hash fragment. This must be implemented for any type that is used as a hash fragment like `#[route("/#:hash_fragment")]`.
///
///
/// **This trait is automatically implemented for any types that implement `FromStr` and `Default`.**
///
/// # Example
///
/// ```rust
/// use dioxus::prelude::*;
///
/// #[derive(Routable, Clone)]
/// #[rustfmt::skip]
/// enum Route {
///     // State is stored in the url hash
///     #[route("/#:url_hash")]
///     Home {
///         url_hash: State,
///     },
/// }
///
/// #[component]
/// fn Home(url_hash: State) -> Element {
///     unimplemented!()
/// }
///
///
/// #[derive(Clone, PartialEq, Default)]
/// struct State {
///     count: usize,
///     other_count: usize
/// }
///
/// // The hash segment will be displayed as a string (this will be url encoded automatically)
/// impl std::fmt::Display for State {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}-{}", self.count, self.other_count)
///     }
/// }
///
/// // We need to parse the hash fragment into a struct from the string (this will be url decoded automatically)
/// impl FromHashFragment for State {
///     fn from_hash_fragment(hash: &str) -> Self {
///         let Some((first, second)) = hash.split_once('-') else {
///             // URL fragment parsing shouldn't fail. You can return a default value if you want
///             return Default::default();
///         };
///
///         let first = first.parse().unwrap();
///         let second = second.parse().unwrap();
///
///         State {
///             count: first,
///             other_count: second,
///         }
///     }
/// }
/// ```
#[rustversion::attr(
    since(1.78.0),
    diagnostic::on_unimplemented(
        message = "`FromHashFragment` is not implemented for `{Self}`",
        label = "hash fragment",
        note = "FromHashFragment is automatically implemented for types that implement `FromStr` and `Default`. You need to either implement FromStr and Default or implement FromHashFragment manually."
    )
)]
pub trait FromHashFragment {
    /// Create an instance of `Self` from a hash fragment.
    fn from_hash_fragment(hash: &str) -> Self;
}

impl<T> FromHashFragment for T
where
    T: FromStr + Default,
    T::Err: std::fmt::Display,
{
    fn from_hash_fragment(hash: &str) -> Self {
        match T::from_str(hash) {
            Ok(value) => value,
            Err(err) => {
                tracing::error!("Failed to parse hash fragment: {}", err);
                Default::default()
            }
        }
    }
}
