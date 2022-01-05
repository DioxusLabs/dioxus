use dioxus_core::ScopeState;

pub struct UseRoute<'a> {
    cur_route: String,
    cx: &'a ScopeState,
}

impl<'a> UseRoute<'a> {
    /// Parse the query part of the URL
    pub fn param<T>(&self, param: &str) -> Option<&T> {
        todo!()
    }

    pub fn nth_segment(&self, n: usize) -> Option<&str> {
        todo!()
    }

    pub fn last_segment(&self) -> Option<&str> {
        todo!()
    }

    /// Parse the segments of the URL, using named parameters (defined in your router)
    pub fn segment<T>(&self, name: &str) -> Option<&T> {
        todo!()
    }
}

pub fn use_route(cx: &ScopeState) -> UseRoute {
    todo!()
}
