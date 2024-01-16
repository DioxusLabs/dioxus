use crate::{
    app::App,
    ipc::{EventData, IpcMethod, UserWindowEvent},
    Config,
};
use dioxus_core::*;
use tao::event::{Event, StartCause, WindowEvent};

/// Launch the WebView and run the event loop, with configuration and root props.
///
/// This will block the main thread, and *must* be spawned on the main thread. This function does not assume any runtime
/// and is equivalent to calling launch_with_props with the tokio feature disabled.
pub fn launch_with_props_blocking<Props: Clone + 'static>(
    dioxus_cfg: CrossPlatformConfig<Props>,
    desktop_config: Config,
) {
    let (event_loop, mut app) = App::new(desktop_config, dioxus_cfg);

    event_loop.run(move |window_event, _, control_flow| {
        app.tick(&window_event);

        match window_event {
            Event::NewEvents(StartCause::Init) => app.handle_start_cause_init(),
            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => app.handle_close_requested(window_id),
                WindowEvent::Destroyed { .. } => app.window_destroyed(window_id),
                _ => {}
            },
            Event::UserEvent(UserWindowEvent(event, id)) => match event {
                EventData::Poll => app.poll_vdom(id),
                EventData::NewWindow => app.handle_new_window(),
                EventData::CloseWindow => app.handle_close_msg(id),
                #[cfg(feature = "hot-reload")]
                EventData::HotReloadEvent(msg) => app.handle_hot_reload_msg(msg),
                EventData::Ipc(msg) => match msg.method() {
                    IpcMethod::FileDialog => app.handle_file_dialog_msg(msg, id),
                    IpcMethod::UserEvent => app.handle_user_event_msg(msg, id),
                    IpcMethod::Query => app.handle_query_msg(msg, id),
                    IpcMethod::BrowserOpen => app.handle_browser_open(msg),
                    IpcMethod::Initialize => app.handle_initialize_msg(id),
                    IpcMethod::Other(_) => {}
                },
            },
            _ => {}
        }

        *control_flow = app.control_flow;
    })
}

/// The desktop renderer platform
pub struct DesktopPlatform;

impl<Props: Clone + 'static> PlatformBuilder<Props> for DesktopPlatform {
    type Config = Config;

    fn launch(config: dioxus_core::CrossPlatformConfig<Props>, platform_config: Self::Config) {
        #[cfg(feature = "tokio")]
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(tokio::task::unconstrained(async move {
                launch_with_props_blocking(config, platform_config)
            }));

        #[cfg(not(feature = "tokio"))]
        launch_with_props_blocking(config, platform_config)
    }
}
