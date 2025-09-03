use std::sync::OnceLock;

/// A custom header that can be set with any value to indicate
/// that the server function client should redirect to a new route.
///
/// This is useful because it allows returning a value from the request,
/// while also indicating that a redirect should follow. This cannot be
/// done with an HTTP `3xx` status code, because the browser will follow
/// that redirect rather than returning the desired data.
pub const REDIRECT_HEADER: &str = "serverfnredirect";

/// A function that will be called if a server function returns a `3xx` status
/// or the [`REDIRECT_HEADER`].
pub type RedirectHook = Box<dyn Fn(&str) + Send + Sync>;

// allowed: not in a public API, and pretty straightforward
#[allow(clippy::type_complexity)]
pub(crate) static REDIRECT_HOOK: OnceLock<RedirectHook> = OnceLock::new();

/// Sets a function that will be called if a server function returns a `3xx` status
/// or the [`REDIRECT_HEADER`]. Returns `Err(_)` if the hook has already been set.
pub fn set_redirect_hook(
    hook: impl Fn(&str) + Send + Sync + 'static,
) -> Result<(), RedirectHook> {
    REDIRECT_HOOK.set(Box::new(hook))
}

/// Calls the hook that has been set by [`set_redirect_hook`] to redirect to `loc`.
pub fn call_redirect_hook(loc: &str) {
    if let Some(hook) = REDIRECT_HOOK.get() {
        hook(loc)
    }
}
