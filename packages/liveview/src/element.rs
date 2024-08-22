use dioxus_core::ElementId;
use dioxus_html::{
    geometry::{PixelsRect, PixelsSize, PixelsVector2D},
    MountedResult, RenderedElementBacking,
};

use crate::query::QueryEngine;

/// A mounted element passed to onmounted events
#[derive(Clone)]
pub struct LiveviewElement {
    id: ElementId,
    query: QueryEngine,
}

impl LiveviewElement {
    pub(crate) fn new(id: ElementId, query: QueryEngine) -> Self {
        Self { id, query }
    }
}

macro_rules! scripted_getter {
    ($meth_name:ident, $script:literal, $output_type:path) => {
        fn $meth_name(
            &self,
        ) -> std::pin::Pin<
            Box<dyn futures_util::Future<Output = dioxus_html::MountedResult<$output_type>>>,
        > {
            let script = format!($script, id = self.id.0);

            let fut = self
                .query
                .new_query::<Option<$output_type>>(&script)
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

impl RenderedElementBacking for LiveviewElement {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    // scripted_getter!(
    //     get_scroll_offset,
    //     "return [window.interpreter.getScrollLeft({id}), window.interpreter.getScrollTop({id})]",
    //     PixelsVector2D
    // );

    // scripted_getter!(
    //     get_scroll_size,
    //     "return [window.interpreter.getScrollWidth({id}), window.interpreter.getScrollHeight({id})]",
    //     PixelsSize
    // );

    // scripted_getter!(
    //     get_client_rect,
    //     "return window.interpreter.getClientRect({id});",
    //     PixelsRect
    // );

    // fn scroll_to(
    //     &self,
    //     behavior: dioxus_html::ScrollBehavior,
    // ) -> std::pin::Pin<Box<dyn futures_util::Future<Output = dioxus_html::MountedResult<()>>>> {
    //     let script = format!(
    //         "return window.interpreter.scrollTo({}, {});",
    //         self.id.0,
    //         serde_json::to_string(&behavior).expect("Failed to serialize ScrollBehavior")
    //     );

    //     let fut = self.query.new_query::<bool>(&script).resolve();
    //     Box::pin(async move {
    //         match fut.await {
    //             Ok(true) => Ok(()),
    //             Ok(false) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
    //                 Box::new(DesktopQueryError::FailedToQuery),
    //             )),
    //             Err(err) => {
    //                 MountedResult::Err(dioxus_html::MountedError::OperationFailed(Box::new(err)))
    //             }
    //         }
    //     })
    // }

    // fn set_focus(
    //     &self,
    //     focus: bool,
    // ) -> std::pin::Pin<Box<dyn futures_util::Future<Output = dioxus_html::MountedResult<()>>>> {
    //     let script = format!(
    //         "return window.interpreter.setFocus({}, {});",
    //         self.id.0, focus
    //     );

    //     let fut = self.query.new_query::<bool>(&script).resolve();

    //     Box::pin(async move {
    //         match fut.await {
    //             Ok(true) => Ok(()),
    //             Ok(false) => MountedResult::Err(dioxus_html::MountedError::OperationFailed(
    //                 Box::new(DesktopQueryError::FailedToQuery),
    //             )),
    //             Err(err) => {
    //                 MountedResult::Err(dioxus_html::MountedError::OperationFailed(Box::new(err)))
    //             }
    //         }
    //     })
    // }
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
