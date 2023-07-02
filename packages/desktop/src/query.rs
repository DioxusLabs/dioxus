use std::{cell::RefCell, rc::Rc};

use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;
use slab::Slab;
use thiserror::Error;
use tokio::{sync::broadcast::error::RecvError, task::JoinHandle};
use wry::webview::WebView;

/// Tracks what query ids are currently active
#[derive(Default, Clone)]
struct SharedSlab {
    slab: Rc<RefCell<Slab<()>>>,
}

const QUEUE_NAME: &str = "__msg_queues";

/// Handles sending and receiving arbitrary queries from the webview. Queries can be resolved non-sequentially, so we use ids to track them.
#[derive(Clone)]
pub(crate) struct QueryEngine {
    sender: Rc<tokio::sync::broadcast::Sender<QueryResult>>,
    active_requests: SharedSlab,
    active_queues: SharedSlab,
}

impl Default for QueryEngine {
    fn default() -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(1000);
        Self {
            sender: Rc::new(sender),
            active_requests: SharedSlab::default(),
            active_queues: SharedSlab::default(),
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
            queue_slab: None,
            queue_id: None,
            thread_handle: None,
        }
    }

    pub fn new_query_with_comm<V: DeserializeOwned>(
        &self,
        script: &str,
        webview: &WebView,
        sender: async_channel::Sender<serde_json::Value>,
    ) -> Query<V> {
        let request_id = self.active_requests.slab.borrow_mut().insert(());
        let queue_id = self.active_queues.slab.borrow_mut().insert(());

        let code = format!(
            r#"
            if (!window.{QUEUE_NAME}) {{
                window.{QUEUE_NAME} = [];
            }}

            let _request_id = {request_id};

            if (!window.{QUEUE_NAME}[{queue_id}]) {{
                window.{QUEUE_NAME}[{queue_id}] = [];
            }}
            let _message_queue = window.{QUEUE_NAME}[{queue_id}];

            {script}
            "#
        );

        if let Err(err) = webview.evaluate_script(&code) {
            log::warn!("Query error: {err}");
        }

        let thread_receiver = self.sender.subscribe();
        let thread_handle: JoinHandle<()> = tokio::spawn(async move {
            let mut receiver = thread_receiver;
            loop {
                if let Ok(result) = receiver.recv().await {
                    if result.id == request_id {
                        _ = sender.send(result.data).await;
                    }
                }
            }
        });

        Query {
            slab: self.active_requests.clone(),
            id: request_id,
            reciever: self.sender.subscribe(),
            phantom: std::marker::PhantomData,
            queue_slab: Some(self.active_queues.clone()),
            queue_id: Some(queue_id),
            thread_handle: Some(thread_handle),
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
    queue_slab: Option<SharedSlab>,
    queue_id: Option<usize>,
    thread_handle: Option<JoinHandle<()>>,
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
                        break V::deserialize(result.data).map_err(QueryError::Deserialize);
                    }
                }
                Err(err) => {
                    break Err(QueryError::Recv(err));
                }
            }
        };

        // Remove the query from the slab
        self.cleanup(None);
        result
    }

    /// Send a message to the query
    pub fn send<S: ToString>(&self, webview: &WebView, message: S) -> Result<(), QueryError> {
        let queue_id = match self.queue_id {
            Some(id) => id,
            None => return Err(QueryError::Send("query is not of comm type".to_string())),
        };

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

        webview
            .evaluate_script(&script)
            .map_err(|e| QueryError::Send(e.to_string()))?;

        Ok(())
    }

    pub fn cleanup(&mut self, webview: Option<&WebView>) {
        if let Some(handle) = &self.thread_handle {
            handle.abort();
        }

        self.slab.slab.borrow_mut().remove(self.id);

        if let Some(queue_slab) = &self.queue_slab {
            let queue_id = self.queue_id.unwrap();

            _ = webview.unwrap().evaluate_script(&format!(
                r#"
                    if (!window.{QUEUE_NAME}) {{
                        window.{QUEUE_NAME} = [];
                    }}

                    if (window.{QUEUE_NAME}[{queue_id}]) {{
                        window.{QUEUE_NAME}[{queue_id}] = [];
                    }}
                "#
            ));
            queue_slab.slab.borrow_mut().remove(queue_id);
        }
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
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct QueryResult {
    id: usize,
    data: Value,
}
