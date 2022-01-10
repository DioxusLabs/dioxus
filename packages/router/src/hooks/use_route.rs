use dioxus_core::ScopeState;
use gloo::history::Location;
use std::rc::Rc;

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

    /// Parse the segments of the URL, using named parameters (defined in your router)
    pub fn segment<T>(&self, name: &str) -> Option<&T> {
        todo!()
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
