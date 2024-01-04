//! Convert a serialized event to an event trigger

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct IpcMessage {
    method: String,
    params: serde_json::Value,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum KnownIpcMethod<'a> {
    FileDialog,
    UserEvent,
    Query,
    BrowserOpen,
    Initialize,
    Other(&'a str),
}

impl IpcMessage {
    pub(crate) fn method(&self) -> KnownIpcMethod {
        match self.method.as_str() {
            // todo: this is a misspelling
            "file_diolog" => KnownIpcMethod::FileDialog,
            "user_event" => KnownIpcMethod::UserEvent,
            "query" => KnownIpcMethod::Query,
            "browser_open" => KnownIpcMethod::BrowserOpen,
            "initialize" => KnownIpcMethod::Initialize,
            _ => KnownIpcMethod::Other(&self.method),
        }
    }

    pub(crate) fn params(self) -> serde_json::Value {
        self.params
    }
}
