//! Convert a serialized event to an event trigger

use dioxus_core::ElementId;
use dioxus_html::events::*;
use serde::{Deserialize, Serialize};
use serde_json::from_value;
use std::any::Any;
use std::rc::Rc;

#[derive(Deserialize, Serialize)]
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

#[derive(Deserialize, Serialize)]
struct ImEvent {
    event: String,
    mounted_dom_id: ElementId,
    contents: serde_json::Value,
}

// pub fn make_synthetic_event(name: &str, val: serde_json::Value) -> Option<Rc<dyn Any>> {
//     // right now we don't support the datatransfer in Drag
//     type DragData = MouseData;
//     type ProgressData = MediaData;

//     Some(res)
// }
