//! server-fn codec just use the axum extractors
//! ie Json<T>, Form<T>, etc
//!
//! Axum gives us:
//! - Json<T>
//! - Form<T>
//! - Multipart<T>
//!
//! We need to build/copy:
//! - Cbor<T>
//! - MsgPack<T>
//! - Postcard<T>
//! - Rkyv<T>
//!
//! Others??
//! - url-encoded GET params?
//! - stream?

use axum::extract::Form; // both req/res
use axum::extract::Json; // both req/res
use axum::extract::Multipart; // req only
