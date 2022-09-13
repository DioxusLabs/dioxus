use crate::RouterService;
use dioxus::core::ScopeState;

/// This hook provides access to the `RouterService` for the app.
pub fn use_router(cx: &ScopeState) -> &RouterService {
    cx.use_hook(|| {
        cx.consume_context::<RouterService>()
            .expect("Cannot call use_route outside the scope of a Router component")
    })
}
