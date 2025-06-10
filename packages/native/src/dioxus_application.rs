use anyrender_vello::VelloWindowRenderer;
use blitz_shell::BlitzApplication;
use dioxus_core::{ScopeId, VirtualDom};
use dioxus_history::{History, MemoryHistory};
use std::{collections::HashSet, rc::Rc};
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

use crate::{
    assets::DioxusNativeNetProvider, contexts::DioxusNativeDocument,
    mutation_writer::MutationWriter, BlitzShellEvent, DioxusDocument, WindowConfig,
};

/// Dioxus-native specific event type
pub enum DioxusNativeEvent {
    /// A hotreload event, basically telling us to update our templates.
    #[cfg(all(
        feature = "hot-reload",
        debug_assertions,
        not(target_os = "android"),
        not(target_os = "ios")
    ))]
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
    pending_vdom: Option<VirtualDom>,
    inner: BlitzApplication<VelloWindowRenderer>,
    proxy: EventLoopProxy<BlitzShellEvent>,
}

impl DioxusNativeApplication {
    pub fn new(proxy: EventLoopProxy<BlitzShellEvent>, vdom: VirtualDom) -> Self {
        Self {
            pending_vdom: Some(vdom),
            inner: BlitzApplication::new(proxy.clone()),
            proxy,
        }
    }

    pub fn add_window(&mut self, window_config: WindowConfig<VelloWindowRenderer>) {
        self.inner.add_window(window_config);
    }

    fn handle_blitz_shell_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: &DioxusNativeEvent,
    ) {
        match event {
            #[cfg(all(
                feature = "hot-reload",
                debug_assertions,
                not(target_os = "android"),
                not(target_os = "ios")
            ))]
            DioxusNativeEvent::DevserverEvent(event) => match event {
                dioxus_devtools::DevserverMsg::HotReload(hotreload_message) => {
                    for window in self.inner.windows.values_mut() {
                        let doc = window.downcast_doc_mut::<DioxusDocument>();
                        dioxus_devtools::apply_changes(&doc.vdom, hotreload_message);
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
            #[cfg(not(all(
                feature = "hot-reload",
                debug_assertions,
                not(target_os = "android"),
                not(target_os = "ios")
            )))]
            _ => {
                let _ = event_loop;
                let _ = event;
            }
        }
    }
}

impl ApplicationHandler<BlitzShellEvent> for DioxusNativeApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        tracing::debug!("Injecting document provider into all windows");
        let vdom = self.pending_vdom.take().unwrap();

        #[cfg(feature = "net")]
        let net_provider = {
            let proxy = self.proxy.clone();
            let net_provider = DioxusNativeNetProvider::shared(proxy);
            Some(net_provider)
        };

        #[cfg(not(feature = "net"))]
        let net_provider = None;

        // Create document + window from the baked virtualdom
        let doc = DioxusDocument::new(vdom, net_provider);
        let window = WindowConfig::new(Box::new(doc) as _);

        // little hack since View::init is not public - fix this once alpha-2 is out
        let old_windows = self.inner.windows.keys().copied().collect::<HashSet<_>>();
        self.add_window(window);
        self.inner.resumed(event_loop);
        let new_windows = self.inner.windows.keys().cloned().collect::<HashSet<_>>();

        // todo(jon): we should actually mess with the pending windows instead of passing along the contexts
        for window_id in new_windows.difference(&old_windows) {
            let window = self.inner.windows.get_mut(window_id).unwrap();
            let doc = window.downcast_doc_mut::<DioxusDocument>();
            doc.vdom.in_runtime(|| {
                let shared: Rc<dyn dioxus_document::Document> =
                    Rc::new(DioxusNativeDocument::new(self.proxy.clone(), *window_id));
                ScopeId::ROOT.provide_context(shared);
            });

            // Add history
            let history_provider: Rc<dyn History> = Rc::new(MemoryHistory::default());
            doc.vdom
                .in_runtime(|| ScopeId::ROOT.provide_context(history_provider));

            // Queue rebuild
            let mut writer = MutationWriter::new(&mut doc.inner, &mut doc.vdom_state);
            doc.vdom.rebuild(&mut writer);
            drop(writer);

            // And then request redraw
            window.request_redraw();
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.inner.suspended(event_loop);
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        self.inner.new_events(event_loop, cause);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.inner.window_event(event_loop, window_id, event);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: BlitzShellEvent) {
        match event {
            BlitzShellEvent::Embedder(event) => {
                if let Some(event) = event.downcast_ref::<DioxusNativeEvent>() {
                    self.handle_blitz_shell_event(event_loop, event);
                }
            }
            event => self.inner.user_event(event_loop, event),
        }
    }
}
