#![allow(unused)]
use base64::Engine;
use dioxus_lib::prelude::{has_context, provide_context, use_hook};
use serialize::serde_to_writable;
use std::{cell::RefCell, io::Cursor, rc::Rc, sync::atomic::AtomicUsize};

use base64::engine::general_purpose::STANDARD;
use serde::{de::DeserializeOwned, Serialize};

pub(crate) mod serialize;

pub(crate) fn use_serialize_context() -> SerializeContext {
    use_hook(serialize_context)
}

pub(crate) fn serialize_context() -> SerializeContext {
    has_context().unwrap_or_else(|| provide_context(SerializeContext::default()))
}
