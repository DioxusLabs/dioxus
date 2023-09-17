use dioxus_core::ElementId;
use dioxus_html::{geometry::euclid::Rect, MountedResult, RenderedElementBacking};

use crate::{desktop_context::DesktopContext, query::QueryEngine};

/// A mounted element passed to onmounted events
pub struct DesktopElement {
    id: ElementId,
    webview: DesktopContext,
    query: QueryEngine,
}

impl DesktopElement {
    pub(crate) fn new(id: ElementId, webview: DesktopContext, query: QueryEngine) -> Self {
        Self { id, webview, query }
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
        let script = format!("return window.interpreter.GetClientRect({});", self.id.0);

        let fut = self
            .query
            .new_query::<Option<Rect<f64, f64>>>(&script, self.webview.clone())
            .resolve();
        Box::pin(async move {
            match fut.await {
                Ok(Some(rect)) => Ok(rect),
                Ok(None) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
                    Box::new(DesktopQueryError::FailedToQuery),
                )),
                Err(err) => {
                    MountedResult::Err(dioxus_html::MountedError::OperationFailed(Box::new(err)))
                }
            }
        })
    }

    fn scroll_to(
        &self,
        behavior: dioxus_html::ScrollBehavior,
    ) -> std::pin::Pin<Box<dyn futures_util::Future<Output = dioxus_html::MountedResult<()>>>> {
        let script = format!(
            "return window.interpreter.ScrollTo({}, {});",
            self.id.0,
            serde_json::to_string(&behavior).expect("Failed to serialize ScrollBehavior")
        );

        let fut = self
            .query
            .new_query::<bool>(&script, self.webview.clone())
            .resolve();
        Box::pin(async move {
            match fut.await {
                Ok(true) => Ok(()),
                Ok(false) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
                    Box::new(DesktopQueryError::FailedToQuery),
                )),
                Err(err) => {
                    MountedResult::Err(dioxus_html::MountedError::OperationFailed(Box::new(err)))
                }
            }
        })
    }

    fn set_focus(
        &self,
        focus: bool,
    ) -> std::pin::Pin<Box<dyn futures_util::Future<Output = dioxus_html::MountedResult<()>>>> {
        let script = format!(
            "return window.interpreter.SetFocus({}, {});",
            self.id.0, focus
        );

        let fut = self
            .query
            .new_query::<bool>(&script, self.webview.clone())
            .resolve();

        Box::pin(async move {
            match fut.await {
                Ok(true) => Ok(()),
                Ok(false) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
                    Box::new(DesktopQueryError::FailedToQuery),
                )),
                Err(err) => {
                    MountedResult::Err(dioxus_html::MountedError::OperationFailed(Box::new(err)))
                }
            }
        })
    }
}

#[derive(Debug)]
enum DesktopQueryError {
    FailedToQuery,
}

impl std::fmt::Display for DesktopQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DesktopQueryError::FailedToQuery => write!(f, "Failed to query the element"),
        }
    }
}

impl std::error::Error for DesktopQueryError {}
