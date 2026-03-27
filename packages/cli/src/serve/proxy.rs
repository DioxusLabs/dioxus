use crate::config::WebProxyConfig;
use crate::TraceSrc;
use crate::{Error, Result};

use anyhow::bail;
use axum::body::Body;
use axum::http::request::Parts;
use axum::{body::Body as MyBody, response::IntoResponse};
use axum::{
    http::StatusCode,
    routing::{any, MethodRouter},
    Router,
};
use hyper::client::conn::http1;
use hyper::header::*;
use hyper::{Request, Response, Uri};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

/// Establish a TCP connection to the backend with retry, then send the HTTP request.
/// This reuses the same TCP connection for both health check and request,
/// and supports streaming request bodies (no buffering).
async fn send_with_retry(
    url: &Uri,
    req: Request<MyBody>,
    handle_error: fn(Error) -> Response<Body>,
) -> std::result::Result<Response<hyper::body::Incoming>, Response<Body>> {
    let host = url.host().unwrap_or("127.0.0.1");
    let port = url.port_u16().unwrap_or(80);
    let addr = format!("{host}:{port}");

    let mut backoff = std::time::Duration::from_millis(100);
    let max_wait = std::time::Duration::from_secs(30);
    let start = std::time::Instant::now();

    // Retry TCP connect until backend is ready
    let stream = loop {
        match TcpStream::connect(&addr).await {
            Ok(stream) => break stream,
            Err(e) => {
                if start.elapsed() >= max_wait {
                    return Err(handle_error(anyhow::anyhow!(
                        "Backend not ready after {max_wait:?}: {e}"
                    )));
                }
                tracing::debug!("Backend not ready, retrying in {backoff:?}...");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(std::time::Duration::from_secs(2));
            }
        }
    };

    // Wrap the TCP stream for hyper
    let io = TokioIo::new(stream);

    // Perform HTTP/1.1 handshake on the same connection
    let (mut sender, conn) = http1::handshake(io)
        .await
        .map_err(|e| handle_error(anyhow::anyhow!("HTTP handshake failed: {e}")))?;

    // Spawn connection driver to keep it alive
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::debug!("Connection closed: {e}");
        }
    });

    // Send request through the established connection (streaming body)
    sender
        .send_request(req)
        .await
        .map_err(|e| handle_error(anyhow::anyhow!("Request failed: {e}")))
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
        bail!(
            "Proxy backend URL must have a non-empty path, e.g. {}/api instead of {}",
            proxy.backend.trim_end_matches('/'),
            proxy.backend
        );
    }

    let method_router = proxy_to(url, false, handle_proxy_error);

    // api/*path
    router = router.route(
        &format!("/{}/{{*path}}", trimmed_path.trim_end_matches('/')),
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
    any(move |parts: Parts, mut req: Request<MyBody>| async move {
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

        let upgrade = req.headers().get(UPGRADE);
        if req.uri().scheme().map(|f| f.as_str()) == Some("ws")
            || req.uri().scheme().map(|f| f.as_str()) == Some("wss")
            || upgrade.is_some_and(|h| h.as_bytes().eq_ignore_ascii_case(b"websocket"))
        {
            return super::proxy_ws::proxy_websocket(parts, req, &url).await;
        }

        if nocache {
            crate::serve::insert_no_cache_headers(req.headers_mut());
        }

        let uri = req.uri().clone();

        // Set Host header for backend (send_with_retry handles TCP connection via url)
        if let Some(authority) = url.authority() {
            req.headers_mut().insert(
                HOST,
                authority
                    .to_string()
                    .parse()
                    .expect("authority is valid header value"),
            );
        }

        // Send with retry - TCP connect retries, then reuses connection for HTTP
        let res = send_with_retry(&url, req, handle_error).await;

        match res {
            Ok(res) => {
                // log assets at a different log level
                if uri.path().starts_with("/assets/")
                    || uri.path().starts_with("/_dioxus/")
                    || uri.path().starts_with("/public/")
                    || uri.path().starts_with("/wasm/")
                {
                    tracing::trace!(dx_src = ?TraceSrc::App(crate::BundleFormat::Server), "[{}] {}", res.status().as_u16(), uri);
                } else {
                    tracing::info!(dx_src = ?TraceSrc::App(crate::BundleFormat::Server), "[{}] {}", res.status().as_u16(), uri);
                }

                Ok(res.into_response())
            }
            Err(err) => {
                tracing::error!(dx_src = ?TraceSrc::App(crate::BundleFormat::Server), "[{}] {}", err.status().as_u16(), uri);
                Err(err)
            }
        }
    })
}

pub(crate) fn handle_proxy_error(e: Error) -> axum::http::Response<axum::body::Body> {
    tracing::error!(dx_src = ?TraceSrc::Dev, "Proxy error: {}", e);
    axum::http::Response::builder()
        .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        .body(axum::body::Body::from(format!(
            "Proxy connection failed: {e:#?}"
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
                "/{*path}",
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
            reqwest::get(format!("http://{server_addr}/api"))
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "backend: /api"
        );

        assert_eq!(
            reqwest::get(format!("http://{server_addr}/api/"))
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
}
