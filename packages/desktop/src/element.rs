use dioxus_core::ElementId;
use dioxus_html::{
    geometry::{PixelsRect, PixelsSize, PixelsVector2D},
    MountedResult, RenderedElementBacking, ScrollBehavior,
};

use crate::desktop_context::DesktopContext;

#[derive(Clone)]
/// A mounted element passed to onmounted events
pub struct DesktopElement {
    id: ElementId,
    webview: DesktopContext,
}

impl DesktopElement {
    pub(crate) fn new(id: ElementId, webview: DesktopContext) -> Self {
        Self { id, webview }
    }
}

pub type EvalFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = MountedResult<T>>>>;

impl RenderedElementBacking for DesktopElement {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_scroll_offset(&self) -> EvalFuture<PixelsVector2D> {
        let id = self.id.0;
        let res = self.webview.eval(format!(
            "return [window.interpreter.getScrollLeft({id}), window.interpreter.getScrollTop({id})]"
        ));
        todo!()
        // Box::pin(res.recv_as())
        // Box::pin(async { Err(dioxus_html::MountedError::NotSupported) })
    }

    fn get_scroll_size(&self) -> EvalFuture<PixelsSize> {
        todo!()
        // Box::pin(async { Err(dioxus_html::MountedError::NotSupported) })
    }

    fn get_client_rect(&self) -> EvalFuture<PixelsRect> {
        todo!()
        // Box::pin(async { Err(dioxus_html::MountedError::NotSupported) })
    }

    fn scroll_to(&self, _behavior: ScrollBehavior) -> EvalFuture<()> {
        todo!()
        // Box::pin(async { Err(dioxus_html::MountedError::NotSupported) })
    }

    fn set_focus(&self, _focus: bool) -> EvalFuture<()> {
        todo!()
        // Box::pin(async { Err(dioxus_html::MountedError::NotSupported) })
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

    //     let fut = self
    //         .query
    //         .new_query::<bool>(&script, self.webview.clone())
    //         .resolve();
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

    //     let fut = self
    //         .query
    //         .new_query::<bool>(&script, self.webview.clone())
    //         .resolve();

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
