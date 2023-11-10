use crate::desktop_context::EventData;
use crate::protocol;
use crate::{desktop_context::UserWindowEvent, Config};
use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
pub use wry;
pub use wry::application as tao;
use wry::application::window::Window;
use wry::webview::{WebContext, WebView, WebViewBuilder};

pub fn build(
    cfg: &mut Config,
    event_loop: &EventLoopWindowTarget<UserWindowEvent>,
    proxy: EventLoopProxy<UserWindowEvent>,
) -> (WebView, WebContext) {
    let builder = cfg.window.clone();
    let window = builder.with_visible(false).build(event_loop).unwrap();
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

    let mut webview = WebViewBuilder::new(window)
        .unwrap()
        .with_transparent(cfg.window.window.transparent)
        .with_url("dioxus://index.html/")
        .unwrap()
        .with_ipc_handler(move |window: &Window, payload: String| {
            // defer the event to the main thread
            if let Ok(message) = serde_json::from_str(&payload) {
                _ = proxy.send_event(UserWindowEvent(EventData::Ipc(message), window.id()));
            }
        })
        .with_custom_protocol(String::from("dioxus"), move |r| {
            protocol::desktop_handler(r, custom_head.clone(), index_file.clone(), &root_name)
        })
        .with_file_drop_handler(move |window, evet| {
            file_handler
                .as_ref()
                .map(|handler| handler(window, evet))
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

    // These are commented out because wry is currently broken in wry
    // let mut web_context = WebContext::new(cfg.data_dir.clone());
    // .with_web_context(&mut web_context);

    for (name, handler) in cfg.protocols.drain(..) {
        webview = webview.with_custom_protocol(name, handler)
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

    (webview.build().unwrap(), web_context)
}
