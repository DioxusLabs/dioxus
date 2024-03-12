use std::{cell::RefCell, rc::Rc};

use futures_util::FutureExt;
use generational_box::{Owner, UnsyncStorage};
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

pub(crate) struct SharedSlab<T = ()> {
    pub(crate) slab: Rc<RefCell<Slab<T>>>,
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

pub(crate) struct QueryEntry {
    channel_sender: tokio::sync::mpsc::UnboundedSender<Value>,
    return_sender: Option<tokio::sync::oneshot::Sender<Value>>,
    pub(crate) owner: Option<Owner<UnsyncStorage>>,
}

const QUEUE_NAME: &str = "__msg_queues";

/// Handles sending and receiving arbitrary queries from the webview. Queries can be resolved non-sequentially, so we use ids to track them.
#[derive(Clone)]
pub(crate) struct QueryEngine {
    pub(crate) active_requests: SharedSlab<QueryEntry>,
    query_tx: tokio::sync::mpsc::UnboundedSender<String>,
}

impl QueryEngine {
    pub(crate) fn new(query_tx: tokio::sync::mpsc::UnboundedSender<String>) -> Self {
        Self {
            active_requests: Default::default(),
            query_tx,
        }
    }

    /// Creates a new query and returns a handle to it. The query will be resolved when the webview returns a result with the same id.
    pub fn new_query<V: DeserializeOwned>(&self, script: &str) -> Query<V> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (return_tx, return_rx) = tokio::sync::oneshot::channel();
        let request_id = self.active_requests.slab.borrow_mut().insert(QueryEntry {
            channel_sender: tx,
            return_sender: Some(return_tx),
            owner: None,
        });

        // start the query
        // We embed the return of the eval in a function so we can send it back to the main thread
        if let Err(err) = self.query_tx.send(format!(
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
            tracing::warn!("Query error: {err}");
        }

        Query {
            query_engine: self.clone(),
            id: request_id,
            receiver: rx,
            return_receiver: Some(return_rx),
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
    query_engine: QueryEngine,
    pub receiver: tokio::sync::mpsc::UnboundedReceiver<Value>,
    pub return_receiver: Option<tokio::sync::oneshot::Receiver<Value>>,
    pub id: usize,
    phantom: std::marker::PhantomData<V>,
}

impl<V: DeserializeOwned> Query<V> {
    /// Resolve the query
    pub async fn resolve(mut self) -> Result<V, QueryError> {
        V::deserialize(self.result().await?).map_err(QueryError::Deserialize)
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

        self.query_engine
            .query_tx
            .send(script)
            .map_err(|e| QueryError::Send(e.to_string()))?;

        Ok(())
    }

    /// Poll the query for a message
    pub fn poll_recv(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<Value, QueryError>> {
        self.receiver
            .poll_recv(cx)
            .map(|result| result.ok_or(QueryError::Recv(RecvError::Closed)))
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

    /// Poll the query for a result
    pub fn poll_result(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<Value, QueryError>> {
        match self.return_receiver.as_mut() {
            Some(receiver) => receiver
                .poll_unpin(cx)
                .map_err(|_| QueryError::Recv(RecvError::Closed)),
            None => std::task::Poll::Ready(Err(QueryError::Finished)),
        }
    }
}

impl<V: DeserializeOwned> Drop for Query<V> {
    fn drop(&mut self) {
        self.query_engine
            .active_requests
            .slab
            .borrow_mut()
            .remove(self.id);
        let queue_id = self.id;

        _ = self.query_engine.query_tx.send(format!(
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
