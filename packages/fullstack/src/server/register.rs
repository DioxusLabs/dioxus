use crate::{
    initialize_server_fn_map, middleware::BoxedService, HybridRequest, HybridResponse,
    LazyServerFnMap, ServerFnObj,
};
use axum::body::Body;
use http::{Method, Response, StatusCode};

static REGISTERED_SERVER_FUNCTIONS: LazyServerFnMap<HybridRequest, HybridResponse> =
    initialize_server_fn_map!(HybridRequest, HybridResponse);

// /// Explicitly register a server function. This is only necessary if you are
// /// running the server in a WASM environment (or a rare environment that the
// /// `inventory` crate won't work in.).
// pub fn register_explicit<T>()
// where
//     T: ServerFn + 'static,
// {
//     REGISTERED_SERVER_FUNCTIONS.insert(
//         (T::PATH.into(), T::METHOD),
//         ServerFnTraitObj::new(T::METHOD, T::PATH, |req| Box::pin(T::run_on_server(req))),
//         // ServerFnTraitObj::new::<T>(|req| Box::pin(T::run_on_server(req))),
//     );
// }

/// The set of all registered server function paths.
pub fn server_fn_paths() -> impl Iterator<Item = (&'static str, Method)> {
    REGISTERED_SERVER_FUNCTIONS
        .iter()
        .map(|item| (item.path(), item.method()))
}

/// An Axum handler that responds to a server function request.
pub async fn handle_server_fn(req: HybridRequest) -> HybridResponse {
    let path = req.uri().path();

    if let Some(mut service) = get_server_fn_service(path, req.req.method().clone()) {
        service.run(req).await
    } else {
        let res = Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(format!(
                "Could not find a server function at the route {path}. \
                     \n\nIt's likely that either\n 1. The API prefix you \
                     specify in the `#[server]` macro doesn't match the \
                     prefix at which your server function handler is mounted, \
                     or \n2. You are on a platform that doesn't support \
                     automatic server function registration and you need to \
                     call ServerFn::register_explicit() on the server \
                     function type, somewhere in your `main` function.",
            )))
            .unwrap();

        HybridResponse { res }
    }
}

/// Returns the server function at the given path as a service that can be modified.
pub fn get_server_fn_service(
    path: &str,
    method: Method,
) -> Option<BoxedService<HybridRequest, HybridResponse>> {
    let key = (path.into(), method);
    REGISTERED_SERVER_FUNCTIONS.get(&key).map(|server_fn| {
        let middleware = (server_fn.middleware)();
        let mut service = server_fn.clone().boxed();
        for middleware in middleware {
            service = middleware.layer(service);
        }
        service
    })
}
