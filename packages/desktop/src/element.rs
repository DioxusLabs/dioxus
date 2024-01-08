use dioxus_core::ElementId;
use dioxus_html::{geometry::euclid::Rect, RenderedElementBacking};

use crate::{desktop_context::DesktopContext, query::QueryEngine};

#[derive(Clone)]
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
    fn id(&self) -> usize {
        self.id.0
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_client_rect(
        &self,
    ) -> std::pin::Pin<
        Box<dyn futures_util::Future<Output = dioxus_html::geometry::euclid::Rect<f64, f64>>>,
    > {
        println!("Get_client_rect");

        let script = format!("return window.interpreter.getClientRect({});", self.id.0);

        let fut = self
            .query
            .new_query::<Option<Rect<f64, f64>>>(&script, self.webview.clone())
            .resolve();
        Box::pin(async move {
            let rect = fut.await.unwrap().unwrap();
            rect
            // match fut.await {
            //     Ok(Some(rect)) => Ok(rect),
            //     Ok(None) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
            //         Box::new(DesktopQueryError::FailedToQuery),
            //     )),
            //     Err(err) => {
            //         MountedResult::Err(dioxus_html::MountedError::OperationFailed(Box::new(err)))
            //     }
            // }
        })
    }

    fn scroll_to(
        &self,
        behavior: dioxus_html::ScrollBehavior,
    ) -> std::pin::Pin<Box<dyn futures_util::Future<Output = ()>>> {
        let script = format!(
            "return window.interpreter.scrollTo({}, {});",
            self.id.0,
            serde_json::to_string(&behavior).expect("Failed to serialize ScrollBehavior")
        );

        let fut = self
            .query
            .new_query::<bool>(&script, self.webview.clone())
            .resolve();
        Box::pin(async move {
            fut.await.unwrap();
            // match fut.await {
            //     Ok(true) => Ok(()),
            //     Ok(false) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
            //         Box::new(DesktopQueryError::FailedToQuery),
            //     )),
            //     Err(err) => {
            //         MountedResult::Err(dioxus_html::MountedError::OperationFailed(Box::new(err)))
            //     }
            // }
            // match fut.await {
            //     Ok(true) => Ok(()),
            //     Ok(false) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
            //         Box::new(DesktopQueryError::FailedToQuery),
            //     )),
            //     Err(err) => {
            //         MountedResult::Err(dioxus_html::MountedError::OperationFailed(Box::new(err)))
            //     }
            // }
        })
    }

    fn set_focus(&self, focus: bool) -> std::pin::Pin<Box<dyn futures_util::Future<Output = ()>>> {
        let script = format!(
            "return window.interpreter.setFocus({}, {});",
            self.id.0, focus
        );

        let fut = self
            .query
            .new_query::<bool>(&script, self.webview.clone())
            .resolve();

        Box::pin(async move {
            fut.await.unwrap();
            // match fut.await {
            //     Ok(true) => Ok(()),
            //     Ok(false) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
            //         Box::new(DesktopQueryError::FailedToQuery),
            //     )),
            //     Err(err) => {
            //         MountedResult::Err(dioxus_html::MountedError::OperationFailed(Box::new(err)))
            //     }
            // }
        })
    }
}
