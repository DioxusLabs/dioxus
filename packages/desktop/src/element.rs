use std::{cell::RefCell, rc::Rc};

use dioxus_core::ElementId;
use dioxus_html::{
    MountedResult, MountedReturn, MountedReturnData, NodeUpdate, NodeUpdateData,
    RenderedElementBacking,
};
use slab::Slab;
use wry::webview::WebView;

/// A mounted element passed to onmounted events
pub struct DesktopElement {
    id: ElementId,
    webview: Rc<WebView>,
    query: QueryEngine,
}

impl DesktopElement {
    pub(crate) fn new(id: ElementId, webview: Rc<WebView>, query: QueryEngine) -> Self {
        Self { id, webview, query }
    }

    /// Get the id of the element
    pub fn id(&self) -> ElementId {
        self.id
    }

    /// Get the webview the element is mounted in
    pub fn webview(&self) -> &Rc<WebView> {
        &self.webview
    }
}

impl RenderedElementBacking for DesktopElement {
    fn get_raw_element(&self) -> dioxus_html::MountedResult<&dyn std::any::Any> {
        Ok(self)
    }

    fn get_client_rect(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn futures_util::Future<
                Output = dioxus_html::MountedResult<dioxus_html::geometry::euclid::Rect<f64, f64>>,
            >,
        >,
    > {
        let fut = self
            .query
            .new_query(self.id, NodeUpdateData::GetClientRect {}, &self.webview)
            .resolve();
        Box::pin(async move {
            match fut.await {
                Some(MountedReturnData::GetClientRect(rect)) => Ok(rect),
                Some(_) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
                    Box::new(DesktopQueryError::MismatchedReturn),
                )),
                None => MountedResult::Err(dioxus_html::MountedError::OperationFailed(Box::new(
                    DesktopQueryError::FailedToQuery,
                ))),
            }
        })
    }

    fn scroll_to(
        &self,
        behavior: dioxus_html::ScrollBehavior,
    ) -> std::pin::Pin<Box<dyn futures_util::Future<Output = dioxus_html::MountedResult<()>>>> {
        let fut = self
            .query
            .new_query(
                self.id,
                NodeUpdateData::ScrollTo { behavior },
                &self.webview,
            )
            .resolve();
        Box::pin(async move {
            match fut.await {
                Some(MountedReturnData::ScrollTo(())) => Ok(()),
                Some(_) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
                    Box::new(DesktopQueryError::MismatchedReturn),
                )),
                None => MountedResult::Err(dioxus_html::MountedError::OperationFailed(Box::new(
                    DesktopQueryError::FailedToQuery,
                ))),
            }
        })
    }

    fn set_focus(
        &self,
        focus: bool,
    ) -> std::pin::Pin<Box<dyn futures_util::Future<Output = dioxus_html::MountedResult<()>>>> {
        let fut = self
            .query
            .new_query(self.id, NodeUpdateData::SetFocus { focus }, &self.webview)
            .resolve();
        Box::pin(async move {
            match fut.await {
                Some(MountedReturnData::SetFocus(())) => Ok(()),
                Some(_) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
                    Box::new(DesktopQueryError::MismatchedReturn),
                )),
                None => MountedResult::Err(dioxus_html::MountedError::OperationFailed(Box::new(
                    DesktopQueryError::FailedToQuery,
                ))),
            }
        })
    }
}

#[derive(Default, Clone)]
struct SharedSlab {
    slab: Rc<RefCell<Slab<()>>>,
}

#[derive(Clone)]
pub(crate) struct QueryEngine {
    sender: Rc<tokio::sync::broadcast::Sender<MountedReturn>>,
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
    fn new_query(&self, id: ElementId, update: NodeUpdateData, webview: &WebView) -> Query {
        let request_id = self.active_requests.slab.borrow_mut().insert(());

        let update = NodeUpdate {
            id: id.0 as u32,
            request_id,
            data: update,
        };

        // start the query
        webview
            .evaluate_script(&format!(
                "window.interpreter.handleNodeUpdate({})",
                serde_json::to_string(&update).unwrap()
            ))
            .unwrap();

        Query {
            slab: self.active_requests.clone(),
            id: request_id,
            reciever: self.sender.subscribe(),
        }
    }

    pub fn send(&self, data: MountedReturn) {
        self.sender.send(data).unwrap();
    }
}

struct Query {
    slab: SharedSlab,
    id: usize,
    reciever: tokio::sync::broadcast::Receiver<MountedReturn>,
}

impl Query {
    async fn resolve(mut self) -> Option<MountedReturnData> {
        let result = loop {
            match self.reciever.recv().await {
                Ok(result) => {
                    if result.id == self.id {
                        break result.data;
                    }
                }
                Err(_) => {
                    break None;
                }
            }
        };

        // Remove the query from the slab
        self.slab.slab.borrow_mut().remove(self.id);

        result
    }
}

#[derive(Debug)]
enum DesktopQueryError {
    FailedToQuery,
    MismatchedReturn,
}

impl std::fmt::Display for DesktopQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DesktopQueryError::FailedToQuery => write!(f, "Failed to query the element"),
            DesktopQueryError::MismatchedReturn => {
                write!(f, "The return type did not match the query")
            }
        }
    }
}

impl std::error::Error for DesktopQueryError {}
