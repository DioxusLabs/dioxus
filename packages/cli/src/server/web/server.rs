use crate::{server::HotReloadState, Result};
use axum::{
    body::Body,
    extract::Extension,
    http::{
        self,
        header::{HeaderName, HeaderValue},
        Method, Response, StatusCode,
    },
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use dioxus_cli_config::{CrateConfig, WebHttpsConfig};
use dioxus_hot_reload::HotReloadRouterExt;
use std::{fs, io, process::Command};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::fs::{ServeDir, ServeFileSystemResponseBody},
    ServiceBuilderExt,
};

/// Sets up and returns a router
pub async fn setup_router(config: CrateConfig, hot_reload: HotReloadState) -> Result<Router> {
    // Setup cors
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any)
        .allow_headers(Any);

    let (coep, coop) = if config.cross_origin_policy {
        (
            HeaderValue::from_static("require-corp"),
            HeaderValue::from_static("same-origin"),
        )
    } else {
        (
            HeaderValue::from_static("unsafe-none"),
            HeaderValue::from_static("unsafe-none"),
        )
    };

    // Create file service
    let file_service_config = config.clone();
    let file_service = ServiceBuilder::new()
        .override_response_header(
            HeaderName::from_static("cross-origin-embedder-policy"),
            coep,
        )
        .override_response_header(HeaderName::from_static("cross-origin-opener-policy"), coop)
        .and_then(move |response| async move { Ok(no_cache(file_service_config, response)) })
        .service(ServeDir::new(config.out_dir()));

    // Setup websocket
    let mut router = Router::new().connect_hot_reload();

    // Setup proxy
    for proxy_config in config.dioxus_config.web.proxy {
        router = super::proxy::add_proxy(router, &proxy_config)?;
    }

    // Route file service
    router = router.fallback(get_service(file_service).handle_error(
        |error: std::convert::Infallible| async move {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unhandled internal error: {}", error),
            )
        },
    ));

    router = if let Some(base_path) = config.dioxus_config.web.app.base_path.clone() {
        let base_path = format!("/{}", base_path.trim_matches('/'));
        Router::new()
            .route(&base_path, axum::routing::any_service(router))
            .fallback(get(move || {
                let base_path = base_path.clone();
                async move { format!("Outside of the base path: {}", base_path) }
            }))
    } else {
        router
    };

    // Setup routes
    router = router
        .layer(cors)
        .layer(Extension(hot_reload.receiver.clone()));

    Ok(router)
}

fn no_cache(
    file_service_config: CrateConfig,
    response: Response<ServeFileSystemResponseBody>,
) -> Response<Body> {
    let mut response = if file_service_config.dioxus_config.web.watcher.index_on_404
        && response.status() == StatusCode::NOT_FOUND
    {
        let body = Body::from(
            // TODO: Cache/memoize this.
            std::fs::read_to_string(file_service_config.out_dir().join("index.html"))
                .ok()
                .unwrap(),
        );
        Response::builder()
            .status(StatusCode::OK)
            .body(body)
            .unwrap()
    } else {
        response.into_response()
    };
    let headers = response.headers_mut();
    headers.insert(
        http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
    headers.insert(http::header::PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(http::header::EXPIRES, HeaderValue::from_static("0"));
    response
}

const DEFAULT_KEY_PATH: &str = "ssl/key.pem";
const DEFAULT_CERT_PATH: &str = "ssl/cert.pem";

/// Returns an enum of rustls config and a bool if mkcert isn't installed
pub async fn get_rustls(config: &CrateConfig) -> Result<Option<RustlsConfig>> {
    let web_config = &config.dioxus_config.web.https;
    if web_config.enabled != Some(true) {
        return Ok(None);
    }

    let (cert_path, key_path) = if let Some(true) = web_config.mkcert {
        // mkcert, use it
        get_rustls_with_mkcert(web_config)?
    } else {
        // if mkcert not specified or false, don't use it
        get_rustls_without_mkcert(web_config)?
    };

    Ok(Some(
        RustlsConfig::from_pem_file(cert_path, key_path).await?,
    ))
}

pub fn get_rustls_with_mkcert(web_config: &WebHttpsConfig) -> Result<(String, String)> {
    // Get paths to store certs, otherwise use ssl/item.pem
    let key_path = web_config
        .key_path
        .clone()
        .unwrap_or(DEFAULT_KEY_PATH.to_string());

    let cert_path = web_config
        .cert_path
        .clone()
        .unwrap_or(DEFAULT_CERT_PATH.to_string());

    // Create ssl directory if using defaults
    if key_path == DEFAULT_KEY_PATH && cert_path == DEFAULT_CERT_PATH {
        _ = fs::create_dir("ssl");
    }

    let cmd = Command::new("mkcert")
        .args([
            "-install",
            "-key-file",
            &key_path,
            "-cert-file",
            &cert_path,
            "localhost",
            "::1",
            "127.0.0.1",
        ])
        .spawn();

    match cmd {
        Err(e) => {
            match e.kind() {
                io::ErrorKind::NotFound => tracing::error!("mkcert is not installed. See https://github.com/FiloSottile/mkcert#installation for installation instructions."),
                e => tracing::error!("an error occured while generating mkcert certificates: {}", e.to_string()),
            };
            return Err("failed to generate mkcert certificates".into());
        }
        Ok(mut cmd) => {
            cmd.wait()?;
        }
    }

    Ok((cert_path, key_path))
}

pub fn get_rustls_without_mkcert(web_config: &WebHttpsConfig) -> Result<(String, String)> {
    // get paths to cert & key
    if let (Some(key), Some(cert)) = (web_config.key_path.clone(), web_config.cert_path.clone()) {
        Ok((cert, key))
    } else {
        // missing cert or key
        Err("https is enabled but cert or key path is missing".into())
    }
}
