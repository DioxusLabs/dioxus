use crate::desktop_context::{DesktopContext, UserWindowEvent};

use dioxus_core::*;
use std::{
    collections::HashMap,
    sync::Arc,
    sync::{atomic::AtomicBool, Mutex},
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

        let pending_edits = edit_queue.clone();
        let desktop_context_proxy = proxy.clone();

        std::thread::spawn(move || {
            // We create the runtime as multithreaded, so you can still "spawn" onto multiple threads
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async move {
                println!("starting vdom");

                let mut dom = VirtualDom::new_with_props(root, props);

                let window_context = DesktopContext::new(desktop_context_proxy);

                dom.base_scope().provide_context(window_context);

                let edits = dom.rebuild();

                // println!("got muts: {:#?}", edits);

                {
                    let mut queue = edit_queue.lock().unwrap();
                    queue.push(serde_json::to_string(&edits.template_mutations).unwrap());
                    queue.push(serde_json::to_string(&edits.edits).unwrap());
                    proxy.send_event(UserWindowEvent::Update).unwrap();
                    drop(queue);
                }

                loop {
                    // todo: add the channel of the event loop in
                    tokio::select! {
                        _ = dom.wait_for_work() => {}
                    }

                    let muts = dom
                        .render_with_deadline(tokio::time::sleep(
                            tokio::time::Duration::from_millis(16),
                        ))
                        .await;

                    {
                        let mut queue = edit_queue.lock().unwrap();

                        queue.push(serde_json::to_string(&muts.template_mutations).unwrap());
                        queue.push(serde_json::to_string(&muts.edits).unwrap());

                        drop(queue);
                    }

                    let _ = proxy.send_event(UserWindowEvent::Update);
                }
            })
        });

        Self {
            pending_edits,
            webviews: HashMap::new(),
            is_ready: Arc::new(AtomicBool::new(false)),
            quit_app_on_close: true,
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

            println!("sending edits {:#?}", new_queue);

            for edit in new_queue.drain(..) {
                view.evaluate_script(&format!("window.interpreter.handleEdits({})", edit))
                    .unwrap();
            }
        }
    }
}
