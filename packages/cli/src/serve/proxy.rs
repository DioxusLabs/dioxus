use crate::config::WebProxyConfig;
use crate::TraceSrc;
use crate::{Error, Result};

use anyhow::{anyhow, Context};
use axum::body::Body;
use axum::{body::Body as MyBody, response::IntoResponse};
use axum::{
    http::StatusCode,
    routing::{any, MethodRouter},
    Router,
};
use hyper::{Request, Response, Uri};
use hyper_util::{
    client::legacy::{self, connect::HttpConnector},
    rt::TokioExecutor,
};

#[derive(Debug, Clone)]
struct ProxyClient {
    inner: legacy::Client<hyper_rustls::HttpsConnector<HttpConnector>, MyBody>,
    url: Uri,
}

impl ProxyClient {
    fn new(url: Uri) -> Self {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .unwrap()
            .https_or_http()
            .enable_all_versions()
            .build();
        Self {
            inner: legacy::Client::builder(TokioExecutor::new()).build(https),
            url,
        }
    }

    async fn send(&self, mut req: Request<MyBody>) -> Result<Response<hyper::body::Incoming>> {
        let mut uri_parts = req.uri().clone().into_parts();
        uri_parts.authority = self.url.authority().cloned();
        uri_parts.scheme = self.url.scheme().cloned();
        *req.uri_mut() = Uri::from_parts(uri_parts).context("Invalid URI parts")?;
        self.inner
            .request(req)
            .await
            .map_err(|err| crate::error::Error::Other(anyhow!(err)))
    }
}

/// Add routes to the router handling the specified proxy config.
///
/// We will proxy requests directed at either:
///
/// - the exact path of the proxy config's backend URL, e.g. /api
/// - the exact path with a trailing slash, e.g. /api/
/// - any subpath of the backend URL, e.g. /api/foo/bar
pub(crate) fn add_proxy(mut router: Router, proxy: &WebProxyConfig) -> Result<Router> {
    let url: Uri = proxy.backend.parse()?;
    let path = url.path().to_string();
    let trimmed_path = path.trim_start_matches('/');

    if trimmed_path.is_empty() {
        return Err(crate::Error::ProxySetup(format!(
            "Proxy backend URL must have a non-empty path, e.g. {}/api instead of {}",
            proxy.backend.trim_end_matches('/'),
            proxy.backend
        )));
    }

    let method_router = proxy_to(url, false, handle_proxy_error);

    // api/*path
    router = router.route(
        &format!("/{}/*path", trimmed_path.trim_end_matches('/')),
        method_router.clone(),
    );

    // /api/
    router = router.route(
        &format!("/{}/", trimmed_path.trim_end_matches('/')),
        method_router.clone(),
    );

    // /api
    router = router.route(
        &format!("/{}", trimmed_path.trim_end_matches('/')),
        method_router,
    );

    Ok(router)
}

pub(crate) fn proxy_to(
    url: Uri,
    nocache: bool,
    handle_error: fn(Error) -> Response<Body>,
) -> MethodRouter {
    let client = ProxyClient::new(url.clone());

    any(move |mut req: Request<MyBody>| async move {
        // Prevent request loops
        if req.headers().get("x-proxied-by-dioxus").is_some() {
            return Err(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(
                    "API is sharing a loopback with the dev server. Try setting a different port on the API config.",
                ))
                .unwrap());
        }

        req.headers_mut().insert(
            "x-proxied-by-dioxus",
            "true".parse().expect("header value is valid"),
        );

        // We have to throw a redirect for ws connections since the upgrade handler will not be called
        // Our _dioxus handler will override this in the default case
        if req.uri().scheme().map(|f| f.as_str()) == Some("ws")
            || req.uri().scheme().map(|f| f.as_str()) == Some("wss")
        {
            let new_host = url.host().unwrap_or("localhost");
            let proxied_uri = format!(
                "{scheme}://{host}:{port}{path_and_query}",
                scheme = req.uri().scheme_str().unwrap_or("ws"),
                port = url.port().unwrap(),
                host = new_host,
                path_and_query = req
                    .uri()
                    .path_and_query()
                    .map(|f| f.to_string())
                    .unwrap_or_default()
            );
            tracing::info!(dx_src = ?TraceSrc::Dev, "Proxied websocket request {req:?} to {proxied_uri}");

            return Ok(axum::response::Redirect::permanent(&proxied_uri).into_response());
        }

        if nocache {
            crate::serve::insert_no_cache_headers(req.headers_mut());
        }

        let uri = req.uri().clone();

        // retry with backoff

        let res = client.send(req).await.map_err(handle_error);

        match res {
            Ok(res) => {
                // log assets at a different log level
                if uri.path().starts_with("/assets")
                    || uri.path().starts_with("/_dioxus")
                    || uri.path().starts_with("/public")
                {
                    tracing::trace!(dx_src = ?TraceSrc::Dev, "[{}] {}", res.status().as_u16(), uri);
                } else {
                    tracing::info!(dx_src = ?TraceSrc::Dev, "[{}] {}", res.status().as_u16(), uri);
                }

                Ok(res.into_response())
            }
            Err(err) => {
                tracing::error!(dx_src = ?TraceSrc::Dev, "[{}] {}", err.status().as_u16(), uri);
                Err(err)
            }
        }
    })
}

fn handle_proxy_error(e: Error) -> axum::http::Response<axum::body::Body> {
    tracing::error!(dx_src = ?TraceSrc::Dev, "Proxy error: {}", e);
    axum::http::Response::builder()
        .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        .body(axum::body::Body::from(format!(
            "Proxy connection failed: {:#?}",
            e
        )))
        .unwrap()
}

#[cfg(test)]
mod test {

    use super::*;

    use axum_server::{Handle, Server};

    async fn setup_servers(mut config: WebProxyConfig) -> String {
        let backend_router =
            Router::new().route(
                "/*path",
                any(|request: axum::extract::Request| async move {
                    format!("backend: {}", request.uri())
                }),
            );

        // The API backend server
        let backend_handle_handle = Handle::new();
        let backend_handle_handle_ = backend_handle_handle.clone();
        tokio::spawn(async move {
            Server::bind("127.0.0.1:0".parse().unwrap())
                .handle(backend_handle_handle_)
                .serve(backend_router.into_make_service())
                .await
                .unwrap();
        });

        // Set the user's config to this dummy API we just built so we can test it
        let backend_addr = backend_handle_handle.listening().await.unwrap();
        config.backend = format!("http://{}{}", backend_addr, config.backend);

        // Now set up our actual filesystem server
        let router = super::add_proxy(Router::new(), &config);
        let server_handle_handle = Handle::new();
        let server_handle_handle_ = server_handle_handle.clone();
        tokio::spawn(async move {
            Server::bind("127.0.0.1:0".parse().unwrap())
                .handle(server_handle_handle_)
                .serve(router.unwrap().into_make_service())
                .await
                .unwrap();
        });

        // Expose *just* the filesystem web server's address
        server_handle_handle.listening().await.unwrap().to_string()
    }

    async fn test_proxy_requests(path: String) {
        let config = WebProxyConfig {
            // Normally this would be an absolute URL including scheme/host/port,
            // but in these tests we need to let the OS choose the port so tests
            // don't conflict, so we'll concatenate the final address and this
            // path together.
            // So in day to day usage, use `http://localhost:8000/api` instead!
            backend: path,
        };

        let server_addr = setup_servers(config).await;

        assert_eq!(
            reqwest::get(format!("http://{}/api", server_addr))
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "backend: /api"
        );

        assert_eq!(
            reqwest::get(format!("http://{}/api/", server_addr))
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "backend: /api/"
        );

        assert_eq!(
            reqwest::get(format!("http://{server_addr}/api/subpath"))
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "backend: /api/subpath"
        );
    }

    #[tokio::test]
    async fn add_proxy() {
        test_proxy_requests("/api".to_string()).await;
    }

    #[tokio::test]
    async fn add_proxy_trailing_slash() {
        test_proxy_requests("/api/".to_string()).await;
    }

    #[test]
    fn add_proxy_empty_path() {
        let config = WebProxyConfig {
            backend: "http://localhost:8000".to_string(),
        };
        let router = super::add_proxy(Router::new(), &config);
        match router.unwrap_err() {
            crate::Error::ProxySetup(e) => {
                assert_eq!(
                    e,
                    "Proxy backend URL must have a non-empty path, e.g. http://localhost:8000/api instead of http://localhost:8000"
                );
            }
            e => panic!("Unexpected error type: {}", e),
        }
    }
}
