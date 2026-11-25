use std::{future::Future, pin::Pin, rc::Rc};

use dioxus_html::{
    MountedError, MountedResult, PointerId, RenderedElementBacking,
    geometry::{PixelsRect, PixelsSize, PixelsVector2D},
};
use dioxus_vdom::ElementId;

use crate::{WeakDesktopContext, desktop_context::DesktopContext, query::QueryEngine};

#[derive(Clone)]
/// A mounted element passed to onmounted events
pub struct DesktopElement {
    id: ElementId,
    webview: WeakDesktopContext,
    query: QueryEngine,
}

impl DesktopElement {
    pub(crate) fn new(id: ElementId, webview: DesktopContext, query: QueryEngine) -> Self {
        let webview = Rc::downgrade(&webview);
        Self { id, webview, query }
    }

    fn query_pointer_capture(
        &self,
        method: &str,
        pointer_id: PointerId,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        let script = format!(
            "return window.interpreter.{method}({}, {});",
            self.id.raw(),
            pointer_id.raw()
        );
        let Some(webview) = self.webview.upgrade() else {
            return Box::pin(async {
                Err(MountedError::OperationFailed(Box::new(
                    DesktopQueryError::FailedToQuery,
                )))
            });
        };
        let query = self.query.new_query::<bool>(&script, webview).resolve();
        Box::pin(async move {
            match query.await {
                Ok(true) => Ok(()),
                Ok(false) => Err(MountedError::OperationFailed(Box::new(
                    DesktopQueryError::FailedToQuery,
                ))),
                Err(err) => Err(MountedError::OperationFailed(Box::new(err))),
            }
        })
    }
}

macro_rules! scripted_getter {
    ($meth_name:ident, $script:literal, $output_type:path) => {
        fn $meth_name(
            &self,
        ) -> std::pin::Pin<
            Box<dyn futures_util::Future<Output = dioxus_html::MountedResult<$output_type>>>,
        > {
            let script = format!($script, id = self.id.raw());
            let webview = self
                .webview
                .upgrade()
                .expect("Webview should be alive if the element is being queried");
            let fut = self
                .query
                .new_query::<Option<$output_type>>(&script, webview)
                .resolve();
            Box::pin(async move {
                match fut.await {
                    Ok(Some(res)) => Ok(res),
                    Ok(None) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
                        Box::new(DesktopQueryError::FailedToQuery),
                    )),
                    Err(err) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
                        Box::new(err),
                    )),
                }
            })
        }
    };
}

impl RenderedElementBacking for DesktopElement {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    scripted_getter!(
        get_scroll_offset,
        "return [window.interpreter.getScrollLeft({id}), window.interpreter.getScrollTop({id})]",
        PixelsVector2D
    );

    scripted_getter!(
        get_scroll_size,
        "return [window.interpreter.getScrollWidth({id}), window.interpreter.getScrollHeight({id})]",
        PixelsSize
    );

    scripted_getter!(
        get_client_rect,
        "return window.interpreter.getClientRect({id});",
        PixelsRect
    );

    fn scroll_to(
        &self,
        options: dioxus_html::ScrollToOptions,
    ) -> std::pin::Pin<Box<dyn futures_util::Future<Output = dioxus_html::MountedResult<()>>>> {
        let script = format!(
            "return window.interpreter.scrollTo({}, {});",
            self.id.raw(),
            serde_json::to_string(&options).expect("Failed to serialize ScrollToOptions")
        );
        let webview = self
            .webview
            .upgrade()
            .expect("Webview should be alive if the element is being queried");
        let fut = self.query.new_query::<bool>(&script, webview).resolve();
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

    fn scroll(
        &self,
        coordinates: PixelsVector2D,
        behavior: dioxus_html::ScrollBehavior,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = MountedResult<()>>>> {
        let script = format!(
            "return window.interpreter.scroll({}, {}, {}, {});",
            self.id.raw(),
            coordinates.x,
            coordinates.y,
            serde_json::to_string(&behavior).expect("Failed to serialize ScrollBehavior")
        );
        let webview = self
            .webview
            .upgrade()
            .expect("Webview should be alive if the element is being queried");
        let fut = self.query.new_query::<bool>(&script, webview).resolve();
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
            "return window.interpreter.setFocus({}, {});",
            self.id.raw(),
            focus
        );
        let webview = self
            .webview
            .upgrade()
            .expect("Webview should be alive if the element is being queried");
        let fut = self.query.new_query::<bool>(&script, webview).resolve();

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

    fn set_pointer_capture(
        &self,
        pointer_id: PointerId,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        self.query_pointer_capture("setPointerCapture", pointer_id)
    }

    fn release_pointer_capture(
        &self,
        pointer_id: PointerId,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        self.query_pointer_capture("releasePointerCapture", pointer_id)
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
