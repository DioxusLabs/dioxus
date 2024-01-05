use std::{rc::Rc, task::Waker};

use crate::edits::{EditQueue, WebviewQueue};
use crate::{
    assets::AssetHandlerRegistry, desktop_context::UserWindowEvent, waker::tao_waker, Config,
    DesktopContext,
};
use crate::{
    desktop_context::{EventData, WindowEventHandlers},
    eval::init_eval,
    shortcut::ShortcutRegistry,
};
use crate::{
    protocol::{self},
    DesktopService,
};
use dioxus_core::VirtualDom;
use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use wry::{WebContext, WebViewBuilder};

pub struct WebviewHandler {
    pub dom: VirtualDom,
    pub desktop_context: DesktopContext,
    pub waker: Waker,

    // Wry assumes the webcontext is alive for the lifetime of the webview.
    // We need to keep the webcontext alive, otherwise the webview will crash
    _web_context: WebContext,
}

pub fn create_new_window(
    mut cfg: Config,
    event_loop: &EventLoopWindowTarget<UserWindowEvent>,
    proxy: &EventLoopProxy<UserWindowEvent>,
    dom: VirtualDom,
    queue: &WebviewQueue,
    event_handlers: &WindowEventHandlers,
    shortcut_manager: ShortcutRegistry,
) -> WebviewHandler {
    let window = cfg.window.clone().build(event_loop).unwrap();

    // TODO: allow users to specify their own menubars, again :/
    if cfg.enable_default_menu_bar {
        use crate::menubar::*;
        build_menu_bar(build_default_menu_bar(), &window);
    }

    let window_id = window.id();
    let file_handler = cfg.file_drop_handler.take();
    let custom_head = cfg.custom_head.clone();
    let index_file = cfg.custom_index.clone();
    let root_name = cfg.root_name.clone();

    // We assume that if the icon is None in cfg, then the user just didnt set it
    if cfg.window.window.window_icon.is_none() {
        window.set_window_icon(Some(
            tao::window::Icon::from_rgba(
                include_bytes!("./assets/default_icon.bin").to_vec(),
                460,
                460,
            )
            .expect("image parse failed"),
        ));
    }

    let mut web_context = WebContext::new(cfg.data_dir.clone());
    let edit_queue = EditQueue::default();
    let headless = !cfg.window.window.visible;
    let asset_handlers = AssetHandlerRegistry::new();
    let asset_handlers_ref = asset_handlers.clone();

    let mut webview = WebViewBuilder::new(&window)
        .with_transparent(cfg.window.window.transparent)
        .with_url("dioxus://index.html/")
        .unwrap()
        .with_ipc_handler({
            let proxy = proxy.clone();
            move |payload: String| {
                // defer the event to the main thread
                if let Ok(message) = serde_json::from_str(&payload) {
                    _ = proxy.send_event(UserWindowEvent(EventData::Ipc(message), window_id));
                }
            }
        })
        .with_asynchronous_custom_protocol(String::from("dioxus"), {
            let edit_queue = edit_queue.clone();
            move |request, responder| {
                let custom_head = custom_head.clone();
                let index_file = index_file.clone();
                let root_name = root_name.clone();
                let asset_handlers_ref = asset_handlers_ref.clone();
                let edit_queue = edit_queue.clone();
                tokio::spawn(async move {
                    protocol::desktop_handler(
                        request,
                        custom_head.clone(),
                        index_file.clone(),
                        &root_name,
                        &asset_handlers_ref,
                        &edit_queue,
                        headless,
                        responder,
                    )
                    .await;
                });
            }
        })
        .with_file_drop_handler(move |event| {
            file_handler
                .as_ref()
                .map(|handler| handler(window_id, event))
                .unwrap_or_default()
        })
        .with_web_context(&mut web_context);

    #[cfg(windows)]
    {
        // Windows has a platform specific settings to disable the browser shortcut keys
        use wry::webview::WebViewBuilderExtWindows;
        webview = webview.with_browser_accelerator_keys(false);
    }

    if let Some(color) = cfg.background_color {
        webview = webview.with_background_color(color);
    }

    for (name, handler) in cfg.protocols.drain(..) {
        webview = webview.with_custom_protocol(name, move |r| handler(r))
    }

    const INITIALIZATION_SCRIPT: &str = r#"
        if (document.addEventListener) {
        document.addEventListener('contextmenu', function(e) {
            e.preventDefault();
        }, false);
        } else {
        document.attachEvent('oncontextmenu', function() {
            window.event.returnValue = false;
        });
        }
    "#;

    if cfg.disable_context_menu {
        // in release mode, we don't want to show the dev tool or reload menus
        webview = webview.with_initialization_script(INITIALIZATION_SCRIPT)
    } else {
        // in debug, we are okay with the reload menu showing and dev tool
        webview = webview.with_devtools(true);
    }

    let webview = webview.build().unwrap();

    let desktop_context = Rc::from(DesktopService::new(
        window,
        webview,
        proxy.clone(),
        event_loop.clone(),
        queue.clone(),
        event_handlers.clone(),
        shortcut_manager,
        edit_queue,
        asset_handlers,
    ));

    dom.base_scope().provide_context(desktop_context.clone());

    init_eval(dom.base_scope());

    WebviewHandler {
        // We want to poll the virtualdom and the event loop at the same time, so the waker will be connected to both
        waker: tao_waker(proxy.clone(), desktop_context.window.id()),
        desktop_context,
        dom,
        _web_context: web_context,
    }
}
