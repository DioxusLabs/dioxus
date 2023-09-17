use crate::{Result, WebProxyConfig};

use anyhow::Context;
use axum::{http::StatusCode, routing::any, Router};
use hyper::{Request, Response, Uri};

#[derive(Debug, Clone)]
struct ProxyClient {
    inner: hyper::Client<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
    url: Uri,
}

impl ProxyClient {
    fn new(url: Uri) -> Self {
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_or_http()
            .enable_http1()
            .build();
        Self {
            inner: hyper::Client::builder().build(https),
            url,
        }
    }

    async fn send(
        &self,
        mut req: Request<hyper::body::Body>,
    ) -> Result<Response<hyper::body::Body>> {
        let mut uri_parts = req.uri().clone().into_parts();
        uri_parts.authority = self.url.authority().cloned();
        uri_parts.scheme = self.url.scheme().cloned();
        *req.uri_mut() = Uri::from_parts(uri_parts).context("Invalid URI parts")?;
        self.inner
            .request(req)
            .await
            .map_err(crate::error::Error::ProxyRequestError)
    }
}

/// Add routes to the router handling the specified proxy config.
///
/// We will proxy requests directed at either:
///
/// - the exact path of the proxy config's backend URL, e.g. /api
/// - the exact path with a trailing slash, e.g. /api/
/// - any subpath of the backend URL, e.g. /api/foo/bar
pub fn add_proxy(mut router: Router, proxy: &WebProxyConfig) -> Result<Router> {
    let url: Uri = proxy.backend.parse()?;
    let path = url.path().to_string();
    let trimmed_path = path.trim_end_matches('/');

    if trimmed_path.is_empty() {
        return Err(crate::Error::ProxySetupError(format!(
            "Proxy backend URL must have a non-empty path, e.g. {}/api instead of {}",
            proxy.backend.trim_end_matches('/'),
            proxy.backend
        )));
    }

    let client = ProxyClient::new(url);

    // We also match everything after the path using a wildcard matcher.
    let wildcard_client = client.clone();

    router = router.route(
        // Always remove trailing /'s so that the exact route
        // matches.
        trimmed_path,
        any(move |req| async move {
            client
                .send(req)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }),
    );

    // Wildcard match anything else _after_ the backend URL's path.
    // Note that we know `path` ends with a trailing `/` in this branch,
    // so `wildcard` will look like `http://localhost/api/*proxywildcard`.
    let wildcard = format!("{}/*proxywildcard", trimmed_path);
    router = router.route(
        &wildcard,
        any(move |req| async move {
            wildcard_client
                .send(req)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }),
    );
    Ok(router)
}

#[cfg(test)]
mod test {

    use super::*;

    use axum::{extract::Path, Router};

    fn setup_servers(
        mut config: WebProxyConfig,
    ) -> (
        tokio::task::JoinHandle<()>,
        tokio::task::JoinHandle<()>,
        String,
    ) {
        let backend_router = Router::new().route(
            "/*path",
            any(|path: Path<String>| async move { format!("backend: {}", path.0) }),
        );
        let backend_server = axum::Server::bind(&"127.0.0.1:0".parse().unwrap())
            .serve(backend_router.into_make_service());
        let backend_addr = backend_server.local_addr();
        let backend_handle = tokio::spawn(async move { backend_server.await.unwrap() });
        config.backend = format!("http://{}{}", backend_addr, config.backend);
        let router = super::add_proxy(Router::new(), &config);
        let server = axum::Server::bind(&"127.0.0.1:0".parse().unwrap())
            .serve(router.unwrap().into_make_service());
        let server_addr = server.local_addr();
        let server_handle = tokio::spawn(async move { server.await.unwrap() });
        (backend_handle, server_handle, server_addr.to_string())
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
        let (backend_handle, server_handle, server_addr) = setup_servers(config);
        let resp = hyper::Client::new()
            .get(format!("http://{}/api", server_addr).parse().unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            hyper::body::to_bytes(resp.into_body()).await.unwrap(),
            "backend: /api"
        );

        let resp = hyper::Client::new()
            .get(format!("http://{}/api/", server_addr).parse().unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            hyper::body::to_bytes(resp.into_body()).await.unwrap(),
            "backend: /api/"
        );

        let resp = hyper::Client::new()
            .get(
                format!("http://{}/api/subpath", server_addr)
                    .parse()
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            hyper::body::to_bytes(resp.into_body()).await.unwrap(),
            "backend: /api/subpath"
        );
        backend_handle.abort();
        server_handle.abort();
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
            crate::Error::ProxySetupError(e) => {
                assert_eq!(
                    e,
                    "Proxy backend URL must have a non-empty path, e.g. http://localhost:8000/api instead of http://localhost:8000"
                );
            }
            e => panic!("Unexpected error type: {}", e),
        }
    }
}
