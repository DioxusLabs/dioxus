use std::{cell::RefCell, rc::Rc};

use crate::DesktopContext;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;
use slab::Slab;
use thiserror::Error;
use tokio::sync::broadcast::error::RecvError;

const DIOXUS_CODE: &str = r#"
let dioxus = {
    recv: function () {
        return new Promise((resolve, _reject) => {
            // Ever 50 ms check for new data
            let timeout = setTimeout(() => {
                let __msg = null;
                while (true) {
                    let __data = _message_queue.shift();
                    if (__data) {
                        __msg = __data;
                        break;
                    }
                }
                clearTimeout(timeout);
                resolve(__msg);
            }, 50);
        });
    },

    send: function (value) {
        window.ipc.postMessage(
            JSON.stringify({
                "method":"query",
                "params": {
                    "id": _request_id,
                    "data": value,
                    "returned_value": false
                }
            })
        );
    }
}"#;

/// Tracks what query ids are currently active

struct SharedSlab<T = ()> {
    slab: Rc<RefCell<Slab<T>>>,
}

impl<T> Clone for SharedSlab<T> {
    fn clone(&self) -> Self {
        Self {
            slab: self.slab.clone(),
        }
    }
}

impl<T> Default for SharedSlab<T> {
    fn default() -> Self {
        SharedSlab {
            slab: Rc::new(RefCell::new(Slab::new())),
        }
    }
}

struct QueryEntry {
    channel_sender: tokio::sync::mpsc::UnboundedSender<Value>,
    return_sender: Option<tokio::sync::oneshot::Sender<Value>>,
}

const QUEUE_NAME: &str = "__msg_queues";

/// Handles sending and receiving arbitrary queries from the webview. Queries can be resolved non-sequentially, so we use ids to track them.
#[derive(Clone, Default)]
pub(crate) struct QueryEngine {
    active_requests: SharedSlab<QueryEntry>,
}

impl QueryEngine {
    /// Creates a new query and returns a handle to it. The query will be resolved when the webview returns a result with the same id.
    pub fn new_query<V: DeserializeOwned>(
        &self,
        script: &str,
        context: DesktopContext,
    ) -> Query<V> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (return_tx, return_rx) = tokio::sync::oneshot::channel();
        let request_id = self.active_requests.slab.borrow_mut().insert(QueryEntry {
            channel_sender: tx,
            return_sender: Some(return_tx),
        });

        // start the query
        // We embed the return of the eval in a function so we can send it back to the main thread
        if let Err(err) = context.webview.evaluate_script(&format!(
            r#"(function(){{
                (async (resolve, _reject) => {{
                    {DIOXUS_CODE}
                    if (!window.{QUEUE_NAME}) {{
                        window.{QUEUE_NAME} = [];
                    }}
    
                    let _request_id = {request_id};
    
                    if (!window.{QUEUE_NAME}[{request_id}]) {{
                        window.{QUEUE_NAME}[{request_id}] = [];
                    }}
                    let _message_queue = window.{QUEUE_NAME}[{request_id}];
    
                    {script}
                }})().then((result)=>{{
                    let returned_value = {{
                        "method":"query",
                        "params": {{
                            "id": {request_id},
                            "data": result,
                            "returned_value": true
                        }}
                    }};
                    window.ipc.postMessage(
                        JSON.stringify(returned_value)
                    );
                }})
            }})();"#
        )) {
            log::warn!("Query error: {err}");
        }

        Query {
            slab: self.active_requests.clone(),
            id: request_id,
            receiver: rx,
            return_receiver: Some(return_rx),
            desktop: context,
            phantom: std::marker::PhantomData,
        }
    }

    /// Send a query channel message to the correct query
    pub fn send(&self, data: QueryResult) {
        let QueryResult {
            id,
            data,
            returned_value,
        } = data;
        let mut slab = self.active_requests.slab.borrow_mut();
        if let Some(entry) = slab.get_mut(id) {
            if returned_value {
                if let Some(sender) = entry.return_sender.take() {
                    let _ = sender.send(data);
                }
            } else {
                let _ = entry.channel_sender.send(data);
            }
        }
    }
}

pub(crate) struct Query<V: DeserializeOwned> {
    desktop: DesktopContext,
    slab: SharedSlab<QueryEntry>,
    receiver: tokio::sync::mpsc::UnboundedReceiver<Value>,
    return_receiver: Option<tokio::sync::oneshot::Receiver<Value>>,
    id: usize,
    phantom: std::marker::PhantomData<V>,
}

impl<V: DeserializeOwned> Query<V> {
    /// Resolve the query
    pub async fn resolve(mut self) -> Result<V, QueryError> {
        match self.receiver.recv().await {
            Some(result) => V::deserialize(result).map_err(QueryError::Deserialize),
            None => Err(QueryError::Recv(RecvError::Closed)),
        }
    }

    /// Send a message to the query
    pub fn send<S: ToString>(&self, message: S) -> Result<(), QueryError> {
        let queue_id = self.id;

        let data = message.to_string();
        let script = format!(
            r#"
            if (!window.{QUEUE_NAME}) {{
                window.{QUEUE_NAME} = [];
            }}

            if (!window.{QUEUE_NAME}[{queue_id}]) {{
                window.{QUEUE_NAME}[{queue_id}] = [];
            }}
            window.{QUEUE_NAME}[{queue_id}].push({data});
            "#
        );

        self.desktop
            .webview
            .evaluate_script(&script)
            .map_err(|e| QueryError::Send(e.to_string()))?;

        Ok(())
    }

    /// Receive a message from the query
    pub async fn recv(&mut self) -> Result<Value, QueryError> {
        self.receiver
            .recv()
            .await
            .ok_or(QueryError::Recv(RecvError::Closed))
    }

    /// Receive the result of the query
    pub async fn result(&mut self) -> Result<Value, QueryError> {
        match self.return_receiver.take() {
            Some(receiver) => receiver
                .await
                .map_err(|_| QueryError::Recv(RecvError::Closed)),
            None => Err(QueryError::Finished),
        }
    }
}

impl<V: DeserializeOwned> Drop for Query<V> {
    fn drop(&mut self) {
        self.slab.slab.borrow_mut().remove(self.id);
        let queue_id = self.id;

        _ = self.desktop.webview.evaluate_script(&format!(
            r#"
            if (!window.{QUEUE_NAME}) {{
                window.{QUEUE_NAME} = [];
            }}

            if (window.{QUEUE_NAME}[{queue_id}]) {{
                window.{QUEUE_NAME}[{queue_id}] = [];
            }}
            "#
        ));
    }
}

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Error receiving query result: {0}")]
    Recv(RecvError),
    #[error("Error sending message to query: {0}")]
    Send(String),
    #[error("Error deserializing query result: {0}")]
    Deserialize(serde_json::Error),
    #[error("Query has already been resolved")]
    Finished,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct QueryResult {
    id: usize,
    data: Value,
    #[serde(default)]
    returned_value: bool,
}
