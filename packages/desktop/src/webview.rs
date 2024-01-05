use crate::desktop_context::{EditQueue, EventData};
use crate::protocol::{self, AssetHandlerRegistry};
use crate::{desktop_context::UserWindowEvent, Config};
use muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use tao::window::Window;
use wry::http::Response;
use wry::{WebContext, WebView, WebViewBuilder};

pub(crate) fn build(
    cfg: &mut Config,
    event_loop: &EventLoopWindowTarget<UserWindowEvent>,
    proxy: EventLoopProxy<UserWindowEvent>,
) -> (WebView, WebContext, AssetHandlerRegistry, EditQueue, Window) {
    let mut builder = cfg.window.clone();

    // TODO: restore the menu bar with muda: https://github.com/tauri-apps/muda/blob/dev/examples/wry.rs
    if cfg.enable_default_menu_bar {
        // builder = builder.with_menu(build_default_menu_bar());
    }

    let window = builder.build(event_loop).unwrap();

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
        .with_ipc_handler(move |payload: String| {
            // defer the event to the main thread
            if let Ok(message) = serde_json::from_str(&payload) {
                _ = proxy.send_event(UserWindowEvent(EventData::Ipc(message), window_id));
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

    if cfg.disable_context_menu {
        // in release mode, we don't want to show the dev tool or reload menus
        webview = webview.with_initialization_script(
            r#"
                        if (document.addEventListener) {
                        document.addEventListener('contextmenu', function(e) {
                            e.preventDefault();
                        }, false);
                        } else {
                        document.attachEvent('oncontextmenu', function() {
                            window.event.returnValue = false;
                        });
                        }
                    "#,
        )
    } else {
        // in debug, we are okay with the reload menu showing and dev tool
        webview = webview.with_devtools(true);
    }

    (
        webview.build().unwrap(),
        web_context,
        asset_handlers,
        edit_queue,
        window,
    )
}

/// Builds a standard menu bar depending on the users platform. It may be used as a starting point
/// to further customize the menu bar and pass it to a [`WindowBuilder`](tao::window::WindowBuilder).
/// > Note: The default menu bar enables macOS shortcuts like cut/copy/paste.
/// > The menu bar differs per platform because of constraints introduced
/// > by [`MenuItem`](tao::menu::MenuItem).
pub fn build_default_menu_bar() -> Menu {
    let menu = Menu::new();

    // since it is uncommon on windows to have an "application menu"
    // we add a "window" menu to be more consistent across platforms with the standard menu
    let window_menu = Submenu::new("Window", true);
    window_menu
        .append_items(&[
            &PredefinedMenuItem::fullscreen(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::maximize(None),
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::close_window(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(None),
        ])
        .unwrap();

    let edit_menu = Submenu::new("Window", true);
    edit_menu
        .append_items(&[
            &PredefinedMenuItem::undo(None),
            &PredefinedMenuItem::redo(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::select_all(None),
        ])
        .unwrap();

    menu.append_items(&[&window_menu, &edit_menu]).unwrap();

    menu
}
