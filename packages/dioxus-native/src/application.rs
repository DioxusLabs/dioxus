use crate::waker::{BlitzEvent, BlitzWindowEvent};

use blitz_dom::DocumentLike;
use std::collections::HashMap;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

use crate::{View, WindowConfig};

pub struct Application<Doc: DocumentLike> {
    rt: tokio::runtime::Runtime,
    windows: HashMap<WindowId, View<Doc>>,
    pending_windows: Vec<WindowConfig<Doc>>,
    proxy: EventLoopProxy<BlitzEvent>,

    #[cfg(all(feature = "menu", not(any(target_os = "android", target_os = "ios"))))]
    menu_channel: muda::MenuEventReceiver,
}

impl<Doc: DocumentLike> Application<Doc> {
    pub fn new(rt: tokio::runtime::Runtime, proxy: EventLoopProxy<BlitzEvent>) -> Self {
        Application {
            windows: HashMap::new(),
            pending_windows: Vec::new(),
            rt,
            proxy,

            #[cfg(all(feature = "menu", not(any(target_os = "android", target_os = "ios"))))]
            menu_channel: muda::MenuEvent::receiver().clone(),
        }
    }

    pub fn add_window(&mut self, window_config: WindowConfig<Doc>) {
        self.pending_windows.push(window_config);
    }
}

impl<Doc: DocumentLike> ApplicationHandler<BlitzEvent> for Application<Doc> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Resume existing windows
        for (_, view) in self.windows.iter_mut() {
            view.resume(&self.rt);
        }

        // Initialise pending windows
        for window_config in self.pending_windows.drain(..) {
            let mut view = View::init(window_config, event_loop, &self.proxy);
            view.resume(&self.rt);
            if !view.renderer.is_active() {
                continue;
            }
            self.windows.insert(view.window_id(), view);
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        for (_, view) in self.windows.iter_mut() {
            view.suspend();
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {
        for window_id in self.windows.keys().copied() {
            _ = self.proxy.send_event(BlitzEvent::Window {
                data: BlitzWindowEvent::Poll,
                window_id,
            });
        }

        #[cfg(all(feature = "menu", not(any(target_os = "android", target_os = "ios"))))]
        if let Ok(event) = self.menu_channel.try_recv() {
            if event.id == muda::MenuId::new("dev.show_layout") {
                for (_, view) in self.windows.iter_mut() {
                    view.devtools.show_layout = !view.devtools.show_layout;
                    view.request_redraw();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // Exit the app when window close is requested. TODO: Only exit when last window is closed.
        if matches!(event, WindowEvent::CloseRequested) {
            event_loop.exit();
            return;
        }

        if let Some(window) = self.windows.get_mut(&window_id) {
            window.handle_winit_event(event);
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: BlitzEvent) {
        match event {
            BlitzEvent::Window { data, window_id } => {
                if let Some(view) = self.windows.get_mut(&window_id) {
                    view.handle_blitz_event(data);
                };
            }

            #[cfg(all(
                feature = "hot-reload",
                debug_assertions,
                not(target_os = "android"),
                not(target_os = "ios")
            ))]
            BlitzEvent::DevserverEvent(event) => match event {
                dioxus_devtools::DevserverMsg::HotReload(hotreload_message) => {
                    for window in self.windows.values_mut() {
                        use crate::documents::DioxusDocument;
                        if let Some(dx_doc) =
                            window.dom.as_any_mut().downcast_mut::<DioxusDocument>()
                        {
                            dioxus_devtools::apply_changes(&dx_doc.vdom, &hotreload_message);
                            window.handle_blitz_event(BlitzWindowEvent::Poll);
                        }
                    }
                }
                dioxus_devtools::DevserverMsg::Shutdown => event_loop.exit(),
                dioxus_devtools::DevserverMsg::FullReloadStart => todo!(),
                dioxus_devtools::DevserverMsg::FullReloadFailed => todo!(),
                dioxus_devtools::DevserverMsg::FullReloadCommand => todo!(),
            },
        }
    }
}
