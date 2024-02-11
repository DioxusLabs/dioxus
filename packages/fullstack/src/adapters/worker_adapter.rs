use server_fn::ServerFunctionRegistry;

use crate::{
    prelude::*, server_context::DioxusServerContext, server_fn::DioxusServerFnRegistry,
    server_fn_service,
};

/// a worker adapter that can be used to run dioxus applications in a worker
pub async fn handle_dioxus_application(
    server_fn_route: &'static str,
    mut req: worker::Request,
    env: worker::Env,
) -> worker::Result<worker::Response> {
    let ls = tokio::task::LocalSet::new();

    let result = async move {
        let path = req
            .path()
            .strip_prefix(server_fn_route)
            .map(|s| s.to_string())
            .unwrap_or(req.path());
        if let Some(func) = DioxusServerFnRegistry::get(&path) {
            let mut service = server_fn_service(DioxusServerContext::default(), func.clone());
            let bytes = req.bytes().await.unwrap();
            let body = hyper::body::Body::from(bytes);
            let req = http::Request::builder()
                .method(req.method().as_ref())
                .uri(req.path())
                .body(body)
                .unwrap();

            match service.run(req).await {
                Ok(rep) => {
                    let status = rep.status().as_u16();
                    let bytes = hyper::body::to_bytes(rep.into_body()).await.unwrap();
                    Ok(worker::Response::from_bytes(bytes.to_vec())
                        .unwrap()
                        .with_status(status))
                }
                Err(e) => Err(worker::Error::from(e.to_string())),
            }
        } else {
            Ok(worker::Response::from_html("Not found")
                .unwrap()
                .with_status(404))
        }
    };

    ls.run_until(result).await
}
