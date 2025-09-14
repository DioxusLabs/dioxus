use crate::{HybridRequest, HybridResponse, ServerFnError};
use axum::extract::Request;
// use crate::{response::ClientRes, HybridError, HybridRequest, HybridResponse};
use bytes::Bytes;
use futures::{Sink, Stream};
use std::{future::Future, sync::OnceLock};

static ROOT_URL: OnceLock<&'static str> = OnceLock::new();

/// Set the root server URL that all server function paths are relative to for the client.
///
/// If this is not set, it defaults to the origin.
pub fn set_server_url(url: &'static str) {
    ROOT_URL.set(url).unwrap();
}

/// Returns the root server URL for all server functions.
pub fn get_server_url() -> &'static str {
    ROOT_URL.get().copied().unwrap_or("")
}
