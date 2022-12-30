use crate::desktop_context::UserWindowEvent;
use futures_util::task::ArcWake;
use std::sync::Arc;
use wry::application::event_loop::EventLoopProxy;

/// Create a waker that will send a poll event to the event loop.
///
/// This lets the VirtualDom "come up for air" and process events while the main thread is blocked by the WebView.
///
/// All other IO lives in the Tokio runtime,
pub fn tao_waker(proxy: &EventLoopProxy<UserWindowEvent>) -> std::task::Waker {
    struct DomHandle(EventLoopProxy<UserWindowEvent>);

    // this should be implemented by most platforms, but ios is missing this until
    // https://github.com/tauri-apps/wry/issues/830 is resolved
    unsafe impl Send for DomHandle {}
    unsafe impl Sync for DomHandle {}

    impl ArcWake for DomHandle {
        fn wake_by_ref(arc_self: &Arc<Self>) {
            _ = arc_self.0.send_event(UserWindowEvent::Poll);
        }
    }

    futures_util::task::waker(Arc::new(DomHandle(proxy.clone())))
}
