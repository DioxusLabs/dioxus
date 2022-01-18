use dioxus_core::ScopeState;
use gloo::history::{HistoryResult, Location};
use serde::de::DeserializeOwned;
use std::{rc::Rc, str::FromStr};

use crate::RouterService;

/// This struct provides is a wrapper around the internal router
/// implementation, with methods for getting information about the current
/// route.
pub struct UseRoute {
    router: Rc<RouterService>,
}

impl UseRoute {
    /// This method simply calls the [`Location::query`] method.
    pub fn query<T>(&self) -> HistoryResult<T>
    where
        T: DeserializeOwned,
    {
        self.current_location().query::<T>()
    }

    /// Returns the nth segment in the path. Paths that end with a slash have
    /// the slash removed before determining the segments. If the path has
    /// fewer segments than `n` then this method returns `None`.
    pub fn nth_segment(&self, n: usize) -> Option<String> {
        let mut segments = self.path_segments();
        let len = segments.len();
        if len - 1 < n {
            return None;
        }
        Some(segments.remove(n))
    }

    /// Returns the last segment in the path. Paths that end with a slash have
    /// the slash removed before determining the segments. The root path, `/`,
    /// will return an empty string.
    pub fn last_segment(&self) -> String {
        let mut segments = self.path_segments();
        segments.remove(segments.len() - 1)
    }

    /// Get the named parameter from the path, as defined in your router. The
    /// value will be parsed into the type specified by `T` by calling
    /// `value.parse::<T>()`. This method returns `None` if the named
    /// parameter does not exist in the current path.
    pub fn segment<T>(&self, name: &str) -> Option<Result<T, T::Err>>
    where
        T: FromStr,
    {
        self.router
            .current_path_params()
            .get(name)
            .and_then(|v| Some(v.parse::<T>()))
    }

    /// Returns the [Location] for the current route.
    pub fn current_location(&self) -> Location {
        self.router.current_location()
    }

    fn path_segments(&self) -> Vec<String> {
        let location = self.router.current_location();
        let path = location.path();
        if path == "/" {
            return vec![String::new()];
        }
        let stripped = &location.path()[1..];
        stripped.split('/').map(str::to_string).collect::<Vec<_>>()
    }
}

/// This hook provides access to information about the current location in the
/// context of a [`Router`]. If this function is called outside of a `Router`
/// component it will panic.
pub fn use_route(cx: &ScopeState) -> UseRoute {
    let router = cx
        .consume_context::<RouterService>()
        .expect("Cannot call use_route outside the scope of a Router component")
        .clone();
    UseRoute { router }
}
