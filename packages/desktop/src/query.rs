use crate::DesktopContext;
use futures_util::{FutureExt, StreamExt};
use generational_box::Owner;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;
use slab::Slab;
use std::{cell::RefCell, rc::Rc};
use thiserror::Error;

/// Tracks what query ids are currently active
pub(crate) struct SharedSlab<T = ()> {
    pub slab: Rc<RefCell<Slab<T>>>,
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
    channel_sender: futures_channel::mpsc::UnboundedSender<Value>,
    return_sender: Option<futures_channel::oneshot::Sender<Result<Value, String>>>,
    pub owner: Option<Owner>,
}

/// Handles sending and receiving arbitrary queries from the webview. Queries can be resolved non-sequentially, so we use ids to track them.
#[derive(Clone, Default)]
pub(crate) struct QueryEngine {
    pub active_requests: SharedSlab<QueryEntry>,
}

impl QueryEngine {
    /// Creates a new query and returns a handle to it. The query will be resolved when the webview returns a result with the same id.
    pub fn new_query<V: DeserializeOwned>(
        &self,
        script: &str,
        context: DesktopContext,
    ) -> Query<V> {
        let (tx, rx) = futures_channel::mpsc::unbounded();
        let (return_tx, return_rx) = futures_channel::oneshot::channel();
        let request_id = self.active_requests.slab.borrow_mut().insert(QueryEntry {
            channel_sender: tx,
            return_sender: Some(return_tx),
            owner: None,
        });

        // start the query
        // We embed the return of the eval in a function so we can send it back to the main thread
        if let Err(err) = context.webview.evaluate_script(&format!(
            r#"(function(){{
                let dioxus = window.createQuery({request_id});
                let post_error = function(err) {{
                    let returned_value = {{
                        "method": "query",
                        "params": {{
                            "id": {request_id},
                            "data": {{
                                "data": err,
                                "method": "return_error"
                            }}
                        }}
                    }};
                    window.ipc.postMessage(
                        JSON.stringify(returned_value)
                    );
                }};
                try {{
                    const AsyncFunction = async function () {{}}.constructor;
                    let promise = (new AsyncFunction("dioxus", {script:?}))(dioxus);
                    promise
                        .then((result)=>{{
                            let returned_value = {{
                                "method": "query",
                                "params": {{
                                    "id": {request_id},
                                    "data": {{
                                        "data": result,
                                        "method": "return"
                                    }}
                                }}
                            }};
                            window.ipc.postMessage(
                                JSON.stringify(returned_value)
                            );
                        }})
                        .catch(err => post_error(`Error running JS: ${{err}}`));
                }} catch (error) {{
                    post_error(`Invalid JS: ${{error}}`);
                }}
            }})();"#
        )) {
            tracing::warn!("Query error: {err}");
        }

        Query {
            id: request_id,
            receiver: rx,
            return_receiver: Some(return_rx),
            desktop: context,
            phantom: std::marker::PhantomData,
        }
    }

    /// Send a query channel message to the correct query
    pub fn send(&self, data: QueryResult) {
        let QueryResult { id, data } = data;
        let mut slab = self.active_requests.slab.borrow_mut();
        if let Some(entry) = slab.get_mut(id) {
            match data {
                QueryResultData::Return { data } => {
                    if let Some(sender) = entry.return_sender.take() {
                        let _ = sender.send(Ok(data.unwrap_or_default()));
                    }
                }
                QueryResultData::ReturnError { data } => {
                    if let Some(sender) = entry.return_sender.take() {
                        let _ = sender.send(Err(data.to_string()));
                    }
                }
                QueryResultData::Drop => {
                    slab.remove(id);
                }
                QueryResultData::Send { data } => {
                    let _ = entry.channel_sender.unbounded_send(data);
                }
            }
        }
    }
}

pub(crate) struct Query<V: DeserializeOwned> {
    desktop: DesktopContext,
    receiver: futures_channel::mpsc::UnboundedReceiver<Value>,
    return_receiver: Option<futures_channel::oneshot::Receiver<Result<Value, String>>>,
    pub id: usize,
    phantom: std::marker::PhantomData<V>,
}

impl<V: DeserializeOwned> Query<V> {
    /// Resolve the query
    pub async fn resolve(mut self) -> Result<V, QueryError> {
        let result = self.result().await?;
        V::deserialize(result).map_err(QueryError::Deserialize)
    }

    /// Send a message to the query
    pub fn send<S: ToString>(&self, message: S) -> Result<(), QueryError> {
        let queue_id = self.id;

        let data = message.to_string();
        let script = format!(r#"window.getQuery({queue_id}).rustSend({data});"#);

        self.desktop
            .webview
            .evaluate_script(&script)
            .map_err(|e| QueryError::Send(e.to_string()))?;

        Ok(())
    }

    /// Poll the query for a message
    pub fn poll_recv(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<Value, QueryError>> {
        self.receiver
            .poll_next_unpin(cx)
            .map(|result| result.ok_or(QueryError::Recv(String::from("Receive channel closed"))))
    }

    /// Receive the result of the query
    pub async fn result(&mut self) -> Result<Value, QueryError> {
        match self.return_receiver.take() {
            Some(receiver) => match receiver.await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(err)) => Err(QueryError::Recv(err)),
                Err(err) => Err(QueryError::Recv(err.to_string())),
            },
            None => Err(QueryError::Finished),
        }
    }

    /// Poll the query for a result
    pub fn poll_result(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<Value, QueryError>> {
        match self.return_receiver.as_mut() {
            Some(receiver) => receiver.poll_unpin(cx).map(|result| match result {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(err)) => Err(QueryError::Recv(err)),
                Err(err) => Err(QueryError::Recv(err.to_string())),
            }),
            None => std::task::Poll::Ready(Err(QueryError::Finished)),
        }
    }
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum QueryError {
    #[error("Error receiving query result: {0}")]
    Recv(String),
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
    data: QueryResultData,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "method")]
enum QueryResultData {
    #[serde(rename = "return")]
    Return { data: Option<Value> },
    #[serde(rename = "return_error")]
    ReturnError { data: Value },
    #[serde(rename = "send")]
    Send { data: Value },
    #[serde(rename = "drop")]
    Drop,
}
