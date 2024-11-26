use futures_util::task::ArcWake;
use std::sync::Arc;
use winit::{event_loop::EventLoopProxy, window::WindowId};

#[cfg(feature = "accessibility")]
use accesskit_winit::Event as AccessibilityEvent;
use accesskit_winit::WindowEvent as AccessibilityWindowEvent;
use blitz_dom::net::Resource;

#[derive(Debug, Clone)]
pub enum BlitzEvent {
    Poll {
        window_id: WindowId,
    },

    ResourceLoad {
        window_id: WindowId,
        data: Resource,
    },

    /// An accessibility event from `accesskit`.
    #[cfg(feature = "accessibility")]
    Accessibility {
        window_id: WindowId,
        data: Arc<AccessibilityWindowEvent>,
    },

    /// A hotreload event, basically telling us to update our templates.
    #[cfg(all(
        feature = "hot-reload",
        debug_assertions,
        not(target_os = "android"),
        not(target_os = "ios")
    ))]
    DevserverEvent(dioxus_devtools::DevserverMsg),
    // NewWindow,
    // CloseWindow,
}
impl From<(WindowId, Resource)> for BlitzEvent {
    fn from((window_id, resource): (WindowId, Resource)) -> Self {
        BlitzEvent::ResourceLoad {
            window_id,
            data: resource,
        }
    }
}

#[cfg(feature = "accessibility")]
impl From<AccessibilityEvent> for BlitzEvent {
    fn from(value: AccessibilityEvent) -> Self {
        Self::Accessibility {
            window_id: value.window_id,
            data: Arc::new(value.window_event),
        }
    }
}

/// Create a waker that will send a poll event to the event loop.
///
/// This lets the VirtualDom "come up for air" and process events while the main thread is blocked by the WebView.
///
/// All other IO lives in the Tokio runtime,
pub fn create_waker(proxy: &EventLoopProxy<BlitzEvent>, id: WindowId) -> std::task::Waker {
    struct DomHandle {
        proxy: EventLoopProxy<BlitzEvent>,
        id: WindowId,
    }

    // this should be implemented by most platforms, but ios is missing this until
    // https://github.com/tauri-apps/wry/issues/830 is resolved
    unsafe impl Send for DomHandle {}
    unsafe impl Sync for DomHandle {}

    impl ArcWake for DomHandle {
        fn wake_by_ref(arc_self: &Arc<Self>) {
            _ = arc_self.proxy.send_event(BlitzEvent::Poll {
                window_id: arc_self.id,
            })
        }
    }

    futures_util::task::waker(Arc::new(DomHandle {
        id,
        proxy: proxy.clone(),
    }))
}
