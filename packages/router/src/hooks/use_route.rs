use crate::{ParsedRoute, RouteContext, RouterContext};
use dioxus::core::{ScopeId, ScopeState};
use std::{borrow::Cow, str::FromStr, sync::Arc};
use url::Url;

/// This hook provides access to information about the current location in the
/// context of a [`Router`]. If this function is called outside of a `Router`
/// component it will panic.
pub fn use_route(cx: &ScopeState) -> &UseRoute {
    let router = cx
        .consume_context::<RouterContext>()
        .expect("Cannot call use_route outside the scope of a Router component");

    let handle = cx.use_hook(|| {
        let route_context = cx.consume_context::<RouteContext>();

        router.subscribe_onchange(cx.scope_id());

        UseRouteListener {
            state: UseRoute {
                route_context,
                route: router.current_location(),
            },
            router,
            scope: cx.scope_id(),
        }
    });

    handle.state.route = handle.router.current_location();

    &handle.state
}

/// A handle to the current location of the router.
pub struct UseRoute {
    pub(crate) route: Arc<ParsedRoute>,

    /// If `use_route` is used inside a `Route` component this has some context otherwise `None`.
    pub(crate) route_context: Option<RouteContext>,
}

impl UseRoute {
    /// Get the underlying [`Url`] of the current location.
    pub fn url(&self) -> &Url {
        &self.route.url
    }

    /// Get the first query parameter given the parameter name.
    ///
    /// If you need to get more than one parameter, use [`query_pairs`] on the [`Url`] instead.
    #[cfg(feature = "query")]
    pub fn query<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        let query = self.url().query()?;
        serde_urlencoded::from_str(query).ok()
    }

    /// Get the first query parameter given the parameter name.
    ///
    /// If you need to get more than one parameter, use [`query_pairs`] on the [`Url`] instead.
    pub fn query_param(&self, param: &str) -> Option<Cow<str>> {
        self.route
            .url
            .query_pairs()
            .find(|(k, _)| k == param)
            .map(|(_, v)| v)
    }

    /// Returns the nth segment in the path. Paths that end with a slash have
    /// the slash removed before determining the segments. If the path has
    /// fewer segments than `n` then this method returns `None`.
    pub fn nth_segment(&self, n: usize) -> Option<&str> {
        self.route.url.path_segments()?.nth(n)
    }

    /// Returns the last segment in the path. Paths that end with a slash have
    /// the slash removed before determining the segments. The root path, `/`,
    /// will return an empty string.
    pub fn last_segment(&self) -> Option<&str> {
        self.route.url.path_segments()?.last()
    }

    /// Get the named parameter from the path, as defined in your router. The
    /// value will be parsed into the type specified by `T` by calling
    /// `value.parse::<T>()`. This method returns `None` if the named
    /// parameter does not exist in the current path.
    pub fn segment(&self, name: &str) -> Option<&str> {
        let total_route = match self.route_context {
            None => self.route.url.path(),
            Some(ref ctx) => &ctx.total_route,
        };

        let index = total_route
            .trim_start_matches('/')
            .split('/')
            .position(|segment| segment.starts_with(':') && &segment[1..] == name)?;

        self.route.url.path_segments()?.nth(index)
    }

    /// Get the named parameter from the path, as defined in your router. The
    /// value will be parsed into the type specified by `T` by calling
    /// `value.parse::<T>()`. This method returns `None` if the named
    /// parameter does not exist in the current path.
    pub fn parse_segment<T>(&self, name: &str) -> Option<Result<T, T::Err>>
    where
        T: FromStr,
    {
        self.segment(name).map(|value| value.parse::<T>())
    }

    /// Get the named parameter from the path, as defined in your router. The
    /// value will be parsed into the type specified by `T` by calling
    /// `value.parse::<T>()`. This method returns `None` if the named
    /// parameter does not exist in the current path.
    pub fn parse_segment_or_404<T>(&self, name: &str) -> Option<T>
    where
        T: FromStr,
    {
        match self.parse_segment(name) {
            Some(Ok(val)) => Some(val),
            _ => {
                // todo: throw a 404
                None
            }
        }
    }
}

// The entire purpose of this struct is to unubscribe this component when it is unmounted.
// The UseRoute can be cloned into async contexts, so we can't rely on its drop to unubscribe.
// Instead, we hide the drop implementation on this private type exclusive to the hook,
// and reveal our cached version of UseRoute to the component.
struct UseRouteListener {
    state: UseRoute,
    router: RouterContext,
    scope: ScopeId,
}

impl Drop for UseRouteListener {
    fn drop(&mut self) {
        self.router.unsubscribe_onchange(self.scope)
    }
}
