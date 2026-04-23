use blitz_shell::{BlitzApplication, BlitzShellProxy, View};
use dioxus_core::{ScopeId, provide_context};
use dioxus_history::{History, MemoryHistory};
use std::rc::Rc;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

#[cfg(target_os = "macos")]
use winit::platform::macos::ApplicationHandlerExtMacOS;

use crate::DioxusNativeWindowRenderer;
use crate::{BlitzShellEvent, DioxusDocument, WindowConfig, contexts::DioxusNativeDocument};

/// Dioxus-native specific event type
pub enum DioxusNativeEvent {
    /// A hotreload event, basically telling us to update our templates.
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    DevserverEvent(dioxus_devtools::DevserverMsg),

    /// Create a new head element from the Link and Title elements
    ///
    /// todo(jon): these should probabkly be synchronous somehow
    CreateHeadElement {
        window: WindowId,
        name: String,
        attributes: Vec<(String, String)>,
        contents: Option<String>,
    },
}

pub struct DioxusNativeApplication {
    pending_window: Option<WindowConfig<DioxusNativeWindowRenderer>>,
    inner: BlitzApplication<DioxusNativeWindowRenderer>,
}

impl DioxusNativeApplication {
    pub fn new(
        proxy: BlitzShellProxy,
        event_queue: std::sync::mpsc::Receiver<BlitzShellEvent>,
        config: WindowConfig<DioxusNativeWindowRenderer>,
    ) -> Self {
        Self {
            pending_window: Some(config),
            inner: BlitzApplication::new(proxy, event_queue),
        }
    }

    pub fn add_window(&mut self, window_config: WindowConfig<DioxusNativeWindowRenderer>) {
        self.inner.add_window(window_config);
    }

    fn handle_dioxus_native_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        event: &DioxusNativeEvent,
    ) {
        match event {
            #[cfg(all(feature = "hot-reload", debug_assertions))]
            DioxusNativeEvent::DevserverEvent(event) => match event {
                dioxus_devtools::DevserverMsg::HotReload(hotreload_message) => {
                    for window in self.inner.windows.values_mut() {
                        let doc = window.downcast_doc_mut::<DioxusDocument>();

                        // Apply changes to vdom
                        dioxus_devtools::apply_changes(&doc.vdom, hotreload_message);

                        // Reload changed assets
                        for asset_path in &hotreload_message.assets {
                            if let Some(url) = asset_path.to_str() {
                                doc.inner.borrow_mut().reload_resource_by_href(url);
                            }
                        }

                        window.poll();
                    }
                }
                dioxus_devtools::DevserverMsg::Shutdown => event_loop.exit(),
                dioxus_devtools::DevserverMsg::FullReloadStart => {}
                dioxus_devtools::DevserverMsg::FullReloadFailed => {}
                dioxus_devtools::DevserverMsg::FullReloadCommand => {}
                _ => {}
            },

            DioxusNativeEvent::CreateHeadElement {
                name,
                attributes,
                contents,
                window,
            } => {
                if let Some(window) = self.inner.windows.get_mut(window) {
                    let doc = window.downcast_doc_mut::<DioxusDocument>();
                    doc.create_head_element(name, attributes, contents);
                    window.poll();
                }
            }

            // Suppress unused variable warning
            #[cfg(not(all(feature = "hot-reload", debug_assertions)))]
            #[allow(unreachable_patterns)]
            _ => {
                let _ = event_loop;
                let _ = event;
            }
        }
    }
}

impl ApplicationHandler for DioxusNativeApplication {
    #[cfg(target_os = "macos")]
    fn macos_handler(&mut self) -> Option<&mut dyn ApplicationHandlerExtMacOS> {
        self.inner.macos_handler()
    }

    fn resumed(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.inner.resumed(event_loop);
    }

    fn suspended(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.inner.suspended(event_loop);
    }

    fn destroy_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.inner.destroy_surfaces(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.inner.about_to_wait(event_loop);
    }

    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        #[cfg(feature = "tracing")]
        tracing::debug!("Injecting document provider into all windows");

        if let Some(config) = self.pending_window.take() {
            let mut window = View::init(config, event_loop, &self.inner.proxy);
            let winit_window = Arc::clone(&window.window);
            let renderer = window.renderer.clone();
            let window_id = window.window_id();
            let doc = window.downcast_doc_mut::<DioxusDocument>();

            doc.vdom.in_scope(ScopeId::ROOT, || {
                let shared: Rc<dyn dioxus_document::Document> = Rc::new(DioxusNativeDocument::new(
                    self.inner.proxy.clone(),
                    window_id,
                ));
                provide_context(shared);
            });

            // Add shell provider
            let shell_provider = doc.inner.borrow().shell_provider.clone();
            doc.vdom
                .in_scope(ScopeId::ROOT, move || provide_context(shell_provider));

            // Add history
            let history_provider: Rc<dyn History> = Rc::new(MemoryHistory::default());
            doc.vdom
                .in_scope(ScopeId::ROOT, move || provide_context(history_provider));

            // Add renderer
            doc.vdom
                .in_scope(ScopeId::ROOT, move || provide_context(renderer));

            // Add winit window
            doc.vdom
                .in_scope(ScopeId::ROOT, move || provide_context(winit_window));

            // Queue rebuild
            doc.initial_build();

            // And then request redraw
            window.request_redraw();

            // todo(jon): we should actually mess with the pending windows instead of passing along the contexts
            self.inner.windows.insert(window_id, window);
        }

        self.inner.can_create_surfaces(event_loop);
    }

    fn new_events(&mut self, event_loop: &dyn ActiveEventLoop, cause: StartCause) {
        self.inner.new_events(event_loop, cause);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.inner.window_event(event_loop, window_id, event);
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        while let Ok(event) = self.inner.event_queue.try_recv() {
            match event {
                BlitzShellEvent::Embedder(event) => {
                    if let Some(event) = event.downcast_ref::<DioxusNativeEvent>() {
                        self.handle_dioxus_native_event(event_loop, event);
                    }
                }
                event => self.inner.handle_blitz_shell_event(event_loop, event),
            }
        }
    }
}
