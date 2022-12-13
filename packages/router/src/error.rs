/// Errors that may occur during routing.
#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    /// A hook that needs access to a router was called inside a component, that has no parent
    /// calling the [`use_router`](crate::hooks::use_router) hook.
    #[error("component needing access not inside router")]
    NotInsideRouter,
}
