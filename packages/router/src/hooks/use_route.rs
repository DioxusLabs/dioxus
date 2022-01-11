use dioxus_core::ScopeState;
use gloo::history::Location;
use std::{rc::Rc, str::FromStr};

use crate::RouterService;

pub struct UseRoute<'a> {
    router: Rc<RouterService>,
    cx: &'a ScopeState,
}

impl<'a> UseRoute<'a> {
    /// Parse the query part of the URL
    pub fn param<T>(&self, param: &str) -> Option<&T> {
        todo!()
    }

    pub fn nth_segment(&self, n: usize) -> Option<String> {
        let mut segments = self.path_segments();
        let len = segments.len();
        if len - 1 < n {
            return None;
        }
        Some(segments.remove(n))
    }

    pub fn last_segment(&self) -> Option<String> {
        let mut segments = self.path_segments();
        let len = segments.len();
        if len == 0 {
            return None;
        }
        Some(segments.remove(len - 1))
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

    pub fn current_location(&self) -> Location {
        self.router.current_location()
    }

    fn path_segments(&self) -> Vec<String> {
        let location = self.router.current_location();
        let stripped = &location.path()[1..];
        stripped.split('/').map(str::to_string).collect::<Vec<_>>()
    }
}

pub fn use_route<'a>(cx: &'a ScopeState) -> UseRoute<'a> {
    let router = cx
        .consume_context::<RouterService>()
        .expect("Cannot call use_route outside the scope of a Router component")
        .clone();
    UseRoute { router, cx }
}
