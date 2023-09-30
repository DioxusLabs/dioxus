use serde::Deserialize;
use std::{fmt, sync::Arc};
use tokio::sync::broadcast::Receiver;

/// Liveview window event engine, for subscribing to window-specific client-side events.
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
        session: String,
        depth: usize,
    },
    #[serde(rename = "popstate")]
    PopState { location: Location, state: String },
}

#[derive(Deserialize, Debug, Clone)]
pub struct Location {
    pub path: String,
    pub search: String,
    pub hash: String,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.path, self.search, self.hash)
    }
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
