use futures_util::task::ArcWake;
use std::sync::Arc;
use wry::application::event_loop::EventLoopProxy;

use crate::desktop_context::UserWindowEvent;

/// Create a waker that will send a poll event to the event loop.
///
/// This lets the VirtualDom "come up for air" and process events while the main thread is blocked by the WebView.
///
/// All other IO lives in the Tokio runtime,
pub fn tao_waker(proxy: &EventLoopProxy<UserWindowEvent>) -> std::task::Waker {
    struct DomHandle(EventLoopProxy<UserWindowEvent>);

    impl ArcWake for DomHandle {
        fn wake_by_ref(arc_self: &Arc<Self>) {
            arc_self.0.send_event(UserWindowEvent::Poll).unwrap();
        }
    }

    futures_util::task::waker(Arc::new(DomHandle(proxy.clone())))
}
