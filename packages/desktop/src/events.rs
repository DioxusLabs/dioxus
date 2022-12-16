//! Convert a serialized event to an event trigger

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct IpcMessage {
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

pub(crate) fn parse_ipc_message(payload: &str) -> Option<IpcMessage> {
    match serde_json::from_str(payload) {
        Ok(message) => Some(message),
        Err(e) => {
            log::error!("could not parse IPC message, error: {}", e);
            None
        }
    }
}
