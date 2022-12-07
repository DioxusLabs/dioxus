use crate::desktop_context::{DesktopContext, UserWindowEvent};
use crate::events::{decode_event, EventMessage};
use dioxus_core::*;
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::StreamExt;
#[cfg(target_os = "ios")]
use objc::runtime::Object;
use std::{
    collections::HashMap,
    sync::Arc,
    sync::{atomic::AtomicBool, Mutex},
    time::Duration,
};
use wry::{
    self,
    application::{event_loop::ControlFlow, event_loop::EventLoopProxy, window::WindowId},
    webview::WebView,
};

pub(super) struct DesktopController {
    pub(super) webviews: HashMap<WindowId, WebView>,
    pub(super) pending_edits: Arc<Mutex<Vec<String>>>,
    pub(super) quit_app_on_close: bool,
    pub(super) is_ready: Arc<AtomicBool>,
    pub(super) proxy: EventLoopProxy<UserWindowEvent>,
    pub(super) event_tx: UnboundedSender<serde_json::Value>,

    #[cfg(target_os = "ios")]
    pub(super) views: Vec<*mut Object>,
}

impl DesktopController {
    // Launch the virtualdom on its own thread managed by tokio
    // returns the desktop state
    pub(super) fn new_on_tokio<P: Send + 'static>(
        root: Component<P>,
        props: P,
        proxy: EventLoopProxy<UserWindowEvent>,
    ) -> Self {
        let edit_queue = Arc::new(Mutex::new(Vec::new()));
        let (event_tx, mut event_rx) = unbounded();
        let proxy2 = proxy.clone();

        let pending_edits = edit_queue.clone();
        let desktop_context_proxy = proxy.clone();

        std::thread::spawn(move || {
            // We create the runtime as multithreaded, so you can still "tokio::spawn" onto multiple threads
            // I'd personally not require tokio to be built-in to Dioxus-Desktop, but the DX is worse without it
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async move {
                let mut dom = VirtualDom::new_with_props(root, props)
                    .with_root_context(DesktopContext::new(desktop_context_proxy));
                {
                    let edits = dom.rebuild();
                    let mut queue = edit_queue.lock().unwrap();
                    queue.push(serde_json::to_string(&edits).unwrap());
                    proxy.send_event(UserWindowEvent::EditsReady).unwrap();
                }

                loop {
                    tokio::select! {
                        _ = dom.wait_for_work() => {}
                        Some(json_value) = event_rx.next() => {
                            if let Ok(value) = serde_json::from_value::<EventMessage>(json_value) {
                                let name = value.event.clone();
                                let el_id = ElementId(value.mounted_dom_id);
                                if let Some(evt) = decode_event(value) {
                                    dom.handle_event(&name,  evt, el_id,  dioxus_html::events::event_bubbles(&name));
                                }
                            }
                        }
                    }

                    let muts = dom
                        .render_with_deadline(tokio::time::sleep(Duration::from_millis(16)))
                        .await;

                    edit_queue.lock().unwrap().push(serde_json::to_string(&muts).unwrap());
                    let _ = proxy.send_event(UserWindowEvent::EditsReady);
                }
            })
        });

        Self {
            pending_edits,
            webviews: HashMap::new(),
            is_ready: Arc::new(AtomicBool::new(false)),
            quit_app_on_close: true,
            proxy: proxy2,
            event_tx,
            #[cfg(target_os = "ios")]
            views: vec![],
        }
    }

    pub(super) fn close_window(&mut self, window_id: WindowId, control_flow: &mut ControlFlow) {
        self.webviews.remove(&window_id);

        if self.webviews.is_empty() && self.quit_app_on_close {
            *control_flow = ControlFlow::Exit;
        }
    }

    pub(super) fn try_load_ready_webviews(&mut self) {
        if self.is_ready.load(std::sync::atomic::Ordering::Relaxed) {
            let mut new_queue = Vec::new();

            {
                let mut queue = self.pending_edits.lock().unwrap();
                std::mem::swap(&mut new_queue, &mut *queue);
            }

            let (_id, view) = self.webviews.iter_mut().next().unwrap();

            for edit in new_queue.drain(..) {
                view.evaluate_script(&format!("window.interpreter.handleEdits({})", edit))
                    .unwrap();
            }
        }
    }

    pub(crate) fn set_template(&self, serialized_template: String) {
        todo!()
    }
}
