use std::future::Future;
use std::pin::Pin;
use futures_util::TryStreamExt;

use server_fn::ServerFunctionRegistry;

use crate::{prelude::*, server_context::DioxusServerContext, server_fn::DioxusServerFnRegistry, server_fn_service};

/// TODO
pub fn handle_dioxus_application(server_fn_route: &'static str) -> Box<dyn FnOnce(worker::Request, worker::Env) -> Pin<Box<dyn Future<Output = worker::Result<worker::Response>>>>>
{
    Box::new(move |mut req: worker::Request, env: worker::Env| Box::pin(async move {
        // tracing::debug!("Request: {:?}", req);
        let path = req.path().strip_prefix(server_fn_route).map(|s| s.to_string()).unwrap_or(req.path());
        tracing::trace!("Path: {:?}", path);
        // tracing::trace!("registered: {:?}", DioxusServerFnRegistry::paths_registered());
        let r = if let Some(func) = DioxusServerFnRegistry::get(&path) {
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
                    // let bytes = (*rep.body()).try_fold(Vec::new(), |mut data, chunk| async move {
                    //     data.extend_from_slice(&chunk);
                    //     Ok(data)
                    // }).await.unwrap();
                    tracing::trace!("Ok");
                    Ok(worker::Response::from_bytes(bytes.to_vec()).unwrap().with_status(status))
                }
                Err(e) => {
                    tracing::trace!("Err");
                    Err(worker::Error::from(e.to_string()))
                }
            }
        } else {
            Ok(worker::Response::from_html("Not found").unwrap().with_status(404))
        };
        r
    }))
}