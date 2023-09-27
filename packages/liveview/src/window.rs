use std::sync::Arc;
use serde::Deserialize;
use tokio::sync::broadcast::Receiver;

/// TODO
#[derive(Clone)]
pub struct Window {
    event_rx: Arc<Receiver<WindowEvent>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "params")]
pub enum WindowEvent {
    #[serde(rename = "load")]
    Load {
        location: Location,
        state: String,
    },
    #[serde(rename = "popstate")]
    PopState {
        location: Location,
        state: String,
    },
}

#[derive(Deserialize, Debug, Clone)]
pub struct Location {
    pub path: String,
    pub search: String,
    pub hash: String,
}

impl Window {
    pub fn new(event_rx: Receiver<WindowEvent>) -> Self {
        Self {
            event_rx: Arc::new(event_rx),
        }
    }

    pub fn subscribe(&self) -> Receiver<WindowEvent> {
        self.event_rx.resubscribe()
    }
}