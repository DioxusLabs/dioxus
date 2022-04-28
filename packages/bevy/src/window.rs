use crate::event::{CustomUserEvent, WebKeyboardEvent};
use bevy::{
    ecs::world::WorldCell,
    math::IVec2,
    utils::HashMap,
    window::{Window, WindowDescriptor, WindowId},
};
use dioxus_core::SchedulerMsg;
use dioxus_desktop::{
    cfg::DesktopConfig,
    controller::DesktopController,
    desktop_context::{UserEvent, UserWindowEvent},
    events::{parse_ipc_message, trigger_from_serialized},
    protocol,
    tao::{
        event_loop::EventLoop,
        window::{Window as TaoWindow, WindowId as TaoWindowId},
    },
    wry::webview::{WebView, WebViewBuilder},
};
use raw_window_handle::HasRawWindowHandle;
use std::fmt::{self, Debug};

#[derive(Default)]
pub struct DioxusWindows {
    windows: HashMap<TaoWindowId, WebView>,
    window_id_to_tao: HashMap<WindowId, TaoWindowId>,
    tao_to_window_id: HashMap<TaoWindowId, WindowId>,
    // Some winit functions, such as `set_window_icon` can only be used from the main thread. If
    // they are used in another thread, the app will hang. This marker ensures `WinitWindows` is
    // only ever accessed with bevy's non-send functions and in NonSend systems.
    _not_send_sync: core::marker::PhantomData<*const ()>,
}

impl Debug for DioxusWindows {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DioxusWindows")
            .field("windonw keys", &self.windows.keys())
            .field("window_id_to_tao", &self.window_id_to_tao)
            .field("tao_to_window_id", &self.tao_to_window_id)
            .finish()
    }
}

impl DioxusWindows {
    pub fn create_window<CoreCommand>(
        &mut self,
        world: &WorldCell,
        window_id: WindowId,
        window_descriptor: &WindowDescriptor,
    ) -> Window
    where
        CoreCommand: 'static + Debug,
    {
        let event_loop = world
            .get_non_send_mut::<EventLoop<UserEvent<CustomUserEvent<CoreCommand>>>>()
            .expect("Insert EventLoop as non send resource");
        let proxy = event_loop.create_proxy();
        let mut desktop = world
            .get_non_send_mut::<DesktopController>()
            .expect("Insert DesktopController as non send resource");
        let mut config = world.get_non_send_mut::<DesktopConfig>().unwrap();
        let window = config.window.clone().build(&event_loop).unwrap();
        let tao_window_id = window.id();
        let (is_ready, sender) = (desktop.is_ready.clone(), desktop.sender.clone());

        let file_handler = config.file_drop_handler.take();

        let resource_dir = config.resource_dir.clone();

        self.window_id_to_tao.insert(window_id, tao_window_id);
        self.tao_to_window_id.insert(tao_window_id, window_id);

        let position = window
            .outer_position()
            .ok()
            .map(|position| IVec2::new(position.x, position.y));
        let inner_size = window.inner_size();
        let scale_factor = window.scale_factor();
        let raw_window_handle = window.raw_window_handle();

        let mut webview = WebViewBuilder::new(window)
            .unwrap()
            .with_transparent(config.window.window.transparent)
            .with_url("dioxus://index.html/")
            .unwrap()
            .with_ipc_handler(move |_window: &TaoWindow, payload: String| {
                parse_ipc_message(&payload)
                    .map(|message| match message.method() {
                        "user_event" => {
                            let event = trigger_from_serialized(message.params());
                            sender.unbounded_send(SchedulerMsg::Event(event)).unwrap();
                        }
                        "keyboard_event" => {
                            let event = WebKeyboardEvent::from_value(message.params());
                            proxy
                                .send_event(UserEvent::CustomEvent(CustomUserEvent::KeyboardInput(
                                    event.to_input(),
                                )))
                                .unwrap();
                            println!("{}", event.key());
                        }
                        "initialize" => {
                            is_ready.store(true, std::sync::atomic::Ordering::Relaxed);
                            let _ =
                                proxy.send_event(UserEvent::WindowEvent(UserWindowEvent::Update));
                        }
                        "browser_open" => {
                            let data = message.params();
                            log::trace!("Open browser: {:?}", data);
                            if let Some(temp) = data.as_object() {
                                if temp.contains_key("href") {
                                    let url = temp.get("href").unwrap().as_str().unwrap();
                                    if let Err(e) = webbrowser::open(url) {
                                        log::error!("Open Browser error: {:?}", e);
                                    }
                                }
                            }
                        }
                        _ => {}
                    })
                    .unwrap_or_else(|| {
                        log::warn!("invalid IPC message received");
                    })
            })
            .with_custom_protocol(String::from("dioxus"), move |r| {
                protocol::desktop_handler(r, resource_dir.clone())
            })
            .with_file_drop_handler(move |window, evet| {
                file_handler
                    .as_ref()
                    .map(|handler| handler(window, evet))
                    .unwrap_or_default()
            });

        for (name, handler) in config.protocols.drain(..) {
            webview = webview.with_custom_protocol(name, handler)
        }

        if config.disable_context_menu {
            // in release mode, we don't want to show the dev tool or reload menus
            webview = webview.with_initialization_script(
                r#"
                        if (document.addEventListener) {
                            document.addEventListener('contextmenu', function(e) {
                                alert("You've tried to open context menu");
                                e.preventDefault();
                            }, false);
                        } else {
                            document.attachEvent('oncontextmenu', function() {
                                alert("You've tried to open context menu");
                                window.event.returnValue = false;
                            });
                        }
                    "#,
            )
        } else {
            // in debug, we are okay with the reload menu showing and dev tool
            webview = webview.with_dev_tool(true);
        }

        desktop
            .webviews
            .insert(tao_window_id, webview.build().unwrap());

        Window::new(
            window_id,
            window_descriptor,
            inner_size.width,
            inner_size.height,
            scale_factor,
            position,
            raw_window_handle,
        )
    }
}
