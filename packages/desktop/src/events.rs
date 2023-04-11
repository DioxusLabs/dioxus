//! Convert a serialized event to an event trigger

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct IpcMessage {
    method: String,
    params: serde_json::Value,
}

impl IpcMessage {
    pub(crate) fn method(&self) -> &str {
        self.method.as_str()
    }

    pub(crate) fn params(self) -> serde_json::Value {
        self.params
    }
}
