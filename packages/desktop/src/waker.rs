use crate::desktop_context::{EventData, UserWindowEvent};
use futures_util::task::ArcWake;
use std::sync::Arc;
use tao::{event_loop::EventLoopProxy, window::WindowId};

/// Create a waker that will send a poll event to the event loop.
///
/// This lets the VirtualDom "come up for air" and process events while the main thread is blocked by the WebView.
///
/// All other IO lives in the Tokio runtime,
pub fn tao_waker(proxy: EventLoopProxy<UserWindowEvent>, id: WindowId) -> std::task::Waker {
    struct DomHandle {
        proxy: EventLoopProxy<UserWindowEvent>,
        id: WindowId,
    }

    // this should be implemented by most platforms, but ios is missing this until
    // https://github.com/tauri-apps/wry/issues/830 is resolved
    unsafe impl Send for DomHandle {}
    unsafe impl Sync for DomHandle {}

    impl ArcWake for DomHandle {
        fn wake_by_ref(arc_self: &Arc<Self>) {
            _ = arc_self
                .proxy
                .send_event(UserWindowEvent(EventData::Poll, arc_self.id));
        }
    }

    futures_util::task::waker(Arc::new(DomHandle { id, proxy }))
}
