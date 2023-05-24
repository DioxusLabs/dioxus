use std::{cell::RefCell, rc::Rc};

use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;
use slab::Slab;
use thiserror::Error;
use tokio::sync::broadcast::error::RecvError;
use wry::webview::WebView;

/// Tracks what query ids are currently active
#[derive(Default, Clone)]
struct SharedSlab {
    slab: Rc<RefCell<Slab<()>>>,
}

/// Handles sending and receiving arbitrary queries from the webview. Queries can be resolved non-sequentially, so we use ids to track them.
#[derive(Clone)]
pub(crate) struct QueryEngine {
    sender: Rc<tokio::sync::broadcast::Sender<QueryResult>>,
    active_requests: SharedSlab,
}

impl Default for QueryEngine {
    fn default() -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(8);
        Self {
            sender: Rc::new(sender),
            active_requests: SharedSlab::default(),
        }
    }
}

impl QueryEngine {
    /// Creates a new query and returns a handle to it. The query will be resolved when the webview returns a result with the same id.
    pub fn new_query<V: DeserializeOwned>(&self, script: &str, webview: &WebView) -> Query<V> {
        let request_id = self.active_requests.slab.borrow_mut().insert(());

        // start the query
        // We embed the return of the eval in a function so we can send it back to the main thread
        if let Err(err) = webview.evaluate_script(&format!(
            r#"window.ipc.postMessage(
                JSON.stringify({{
                    "method":"query",
                    "params": {{
                        "id": {request_id},
                        "data": (function(){{{script}}})()
                    }}
                }})
            );"#
        )) {
            log::warn!("Query error: {err}");
        }

        Query {
            slab: self.active_requests.clone(),
            id: request_id,
            reciever: self.sender.subscribe(),
            phantom: std::marker::PhantomData,
        }
    }

    /// Send a query result
    pub fn send(&self, data: QueryResult) {
        let _ = self.sender.send(data);
    }
}

pub(crate) struct Query<V: DeserializeOwned> {
    slab: SharedSlab,
    id: usize,
    reciever: tokio::sync::broadcast::Receiver<QueryResult>,
    phantom: std::marker::PhantomData<V>,
}

impl<V: DeserializeOwned> Query<V> {
    /// Resolve the query
    pub async fn resolve(mut self) -> Result<V, QueryError> {
        let result = loop {
            match self.reciever.recv().await {
                Ok(result) => {
                    if result.id == self.id {
                        break V::deserialize(result.data).map_err(QueryError::DeserializeError);
                    }
                }
                Err(err) => {
                    break Err(QueryError::RecvError(err));
                }
            }
        };

        // Remove the query from the slab
        self.slab.slab.borrow_mut().remove(self.id);

        result
    }
}

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Error receiving query result: {0}")]
    RecvError(RecvError),
    #[error("Error deserializing query result: {0}")]
    DeserializeError(serde_json::Error),
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct QueryResult {
    id: usize,
    data: Value,
}
