//! HTTP clients for dx's own requests.
//!
//! dx enables reqwest's `rustls-no-provider` feature and configures TLS itself: _ring_ with the
//! bundled webpki roots, rather than reqwest's default of aws-lc and the platform verifier.

use std::sync::{Arc, LazyLock};

/// Installs _ring_ as the process-wide rustls crypto provider, unless one is already installed.
///
/// Clients that don't use [`tls_config`] (for example ones that accept invalid local certs) fall
/// back to the process-wide provider, and reqwest panics if none is installed.
pub(crate) fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn tls_config() -> rustls::ClientConfig {
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("ring supports the default TLS versions")
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    config
}

/// A client builder with dx's TLS settings applied.
pub(crate) fn builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder().tls_backend_preconfigured(tls_config())
}

/// The shared async client for downloads and API requests.
pub(crate) fn client() -> &'static reqwest::Client {
    static CLIENT: LazyLock<reqwest::Client> =
        LazyLock::new(|| builder().build().expect("failed to build HTTP client"));
    &CLIENT
}

/// A blocking client with dx's TLS settings, for use off the async runtime (e.g. self-update).
pub(crate) fn blocking_client() -> reqwest::Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .tls_backend_preconfigured(tls_config())
        .build()
}
