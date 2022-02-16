use dioxus_core::{ScopeId, ScopeState};
use serde::de::DeserializeOwned;
use std::{rc::Rc, str::FromStr};

use crate::RouterService;

/// This struct provides is a wrapper around the internal router
/// implementation, with methods for getting information about the current
/// route.
#[derive(Clone)]
pub struct UseRoute {
    router: Rc<RouterService>,
}

impl UseRoute {
    /// This method simply calls the [`Location::query`] method.
    pub fn query<T>(&self) -> crate::Result<T>
    where
        T: DeserializeOwned,
    {
        todo!()
        // self.current_location().query::<T>()
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

    /// Returns the [Location] for the current route as a `&str`.
    pub fn current_location(&self) -> Rc<str> {
        self.router.current_location()
    }

    pub fn native_location<T: 'static>(&self) -> Option<Box<T>> {
        self.router.native_location()
    }

    fn path_segments(&self) -> Vec<String> {
        match self.router.current_location().as_ref() {
            "/" => vec![String::new()],
            location => location[1..]
                .split('/')
                .map(str::to_string)
                .collect::<Vec<_>>(),
        }
    }
}

/// This hook provides access to information about the current location in the
/// context of a [`Router`]. If this function is called outside of a `Router`
/// component it will panic.
pub fn use_route(cx: &ScopeState) -> &UseRoute {
    &cx.use_hook(|_| {
        let router = cx
            .consume_context::<RouterService>()
            .expect("Cannot call use_route outside the scope of a Router component");

        router.subscribe_onchange(cx.scope_id());

        UseRouteListener {
            router: UseRoute { router },
            scope: cx.scope_id(),
        }
    })
    .router
}

// The entire purpose of this struct is to unubscribe this component when it is unmounted.
// The UseRoute can be cloned into async contexts, so we can't rely on its drop to unubscribe.
// Instead, we hide the drop implementation on this private type exclusive to the hook,
// and reveal our cached version of UseRoute to the component.
struct UseRouteListener {
    router: UseRoute,
    scope: ScopeId,
}
impl Drop for UseRouteListener {
    fn drop(&mut self) {
        self.router.router.unsubscribe_onchange(self.scope)
    }
}

/// This hook provides access to the `RouterService` for the app.
pub fn use_router(cx: &ScopeState) -> &Rc<RouterService> {
    cx.use_hook(|_| {
        cx.consume_context::<RouterService>()
            .expect("Cannot call use_route outside the scope of a Router component")
    })
}
