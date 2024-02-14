use server_fn::ServerFunctionRegistry;
use std::sync::{Arc, RwLock};

use dioxus_lib::prelude::VirtualDom;

use crate::{
    prelude::*, server_context::DioxusServerContext, server_fn::DioxusServerFnRegistry,
    server_fn_service,
};

/// a worker adapter that can be used to run dioxus applications in a worker
pub async fn handle_dioxus_application(
    server_fn_route: &'static str,
    cfg: impl Into<ServeConfig>,
    build_virtual_dom: impl Fn() -> VirtualDom + Send + Sync + 'static,
    mut req: worker::Request,
    env: worker::Env,
) -> worker::Result<worker::Response> {
    let ls = tokio::task::LocalSet::new();

    let path = req.path().clone();
    let func_path = path
        .strip_prefix(server_fn_route)
        .map(|s| s.to_string())
        .unwrap_or(path.clone());

    let request = request_workers_to_hyper(req).await?;

    tracing::info!("Handling request: {:?}", request);
    let result = async move {
        if let Some(func) = DioxusServerFnRegistry::get(&func_path) {
            tracing::info!("Running server function: {:?}", func_path);
            let mut service = server_fn_service(DioxusServerContext::default(), func.clone());
            match service.run(request).await {
                Ok(rep) => Ok(response_hyper_to_workers(rep).await),
                Err(e) => Err(worker::Error::from(e.to_string())),
            }
        } else if path.starts_with("/_dioxus/") {
            Ok(worker::Response::from_html(
                "<!DOCTYPE html><html><head><title>Not Found</title></head><body><h1>Not Found</h1></body></html>"
            ).unwrap().with_status(404))
        } else {
            tracing::info!("Rendering page: {:?}", path);
            let cfg = cfg.into();
            let ssr_state = SSRState::new(&cfg);

            render_handler(cfg, ssr_state, Arc::new(build_virtual_dom), request).await
        }
    };

    ls.run_until(result).await
}

async fn render_handler(
    cfg: ServeConfig,
    ssr_state: SSRState,
    virtual_dom_factory: Arc<dyn Fn() -> VirtualDom + Send + Sync>,
    request: http::Request<hyper::Body>,
) -> worker::Result<worker::Response> {
    let (parts, _) = request.into_parts();
    let url = parts.uri.path_and_query().unwrap().to_string();
    let parts: Arc<RwLock<http::request::Parts>> = Arc::new(RwLock::new(parts.into()));
    let server_context = DioxusServerContext::new(parts.clone());

    match ssr_state
        .render(url, &cfg, move || virtual_dom_factory(), &server_context)
        .await
    {
        Ok(rendered) => {
            let crate::render::RenderResponse { html, freshness } = rendered;

            let mut response = http::Response::new(hyper::Body::from(html));
            freshness.write(response.headers_mut());

            let headers = server_context.response_parts().unwrap().headers.clone();
            let mut_headers = response.headers_mut();
            for (key, value) in headers.iter() {
                mut_headers.insert(key, value.clone());
            }

            Ok(response_hyper_to_workers(response).await)
        }
        Err(e) => {
            tracing::error!("Failed to render page: {:?}", e);
            Err(worker::Error::from(e.to_string()))
        }
    }
}

async fn request_workers_to_hyper(
    mut req: worker::Request,
) -> worker::Result<http::Request<hyper::Body>> {
    let builder = http::Request::builder().method(req.method().as_ref());
    let builder = match req.url() {
        Ok(url) => builder.uri(url.to_string()),
        Err(e) => return Err(e),
    };

    // TODO: use req.stream() to stream the body
    match req.bytes().await {
        Ok(v) => builder
            .body(hyper::Body::from(v))
            .map_err(|e| worker::Error::from(e.to_string())),
        Err(worker::Error::JsError(_)) => builder
            .body(hyper::Body::empty())
            .map_err(|e| worker::Error::from(e.to_string())),
        Err(e) => Err(e),
    }
}

async fn response_hyper_to_workers(rep: http::Response<hyper::Body>) -> worker::Response {
    // TODO: use worker::Response::from_stream() to stream the body
    let mut headers = worker::Headers::new();
    for (key, value) in rep.headers().iter() {
        headers
            .append(key.as_str(), value.to_str().unwrap())
            .unwrap();
    }
    let status = rep.status().as_u16();
    let bytes = hyper::body::to_bytes(rep.into_body()).await.unwrap();
    worker::Response::from_bytes(bytes.to_vec())
        .unwrap()
        .with_status(status)
        .with_headers(headers)
}
