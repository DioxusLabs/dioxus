use crate::Config;
use crate::{app::App, ipc::UserWindowEvent};
use dioxus_core::*;
use std::any::Any;
use winit::application::ApplicationHandler;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

/// Launch the WebView and run the event loop, with configuration and root props.
///
/// This will block the main thread, and *must* be spawned on the main thread. This function does not assume any runtime
/// and is equivalent to calling launch_with_props with the tokio feature disabled.
pub fn launch_virtual_dom_blocking(virtual_dom: VirtualDom, desktop_config: Config) -> ! {
    let event_loop = EventLoop::<UserWindowEvent>::with_user_event()
        .build()
        .unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy, desktop_config, virtual_dom);
    event_loop.run_app(&mut app).unwrap();

    std::process::exit(0);
}

impl ApplicationHandler<UserWindowEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.handle_app_resume(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.handle_event(event_loop, Event::WindowEvent { window_id, event });
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserWindowEvent) {
        self.handle_event(event_loop, Event::UserEvent(event));
    }
}

/// Launches the WebView and runs the event loop, with configuration and root props.
pub fn launch_virtual_dom(virtual_dom: VirtualDom, desktop_config: Config) -> ! {
    #[cfg(feature = "tokio_runtime")]
    {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(tokio::task::unconstrained(async move {
                launch_virtual_dom_blocking(virtual_dom, desktop_config)
            }));

        unreachable!("The desktop launch function will never exit")
    }

    #[cfg(not(feature = "tokio_runtime"))]
    {
        launch_virtual_dom_blocking(virtual_dom, desktop_config);
    }
}

/// Launches the WebView and runs the event loop, with configuration and root props.
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Vec<Box<dyn Any>>,
) -> ! {
    let mut virtual_dom = VirtualDom::new(root);

    for context in contexts {
        virtual_dom.insert_any_root_context(context());
    }

    let platform_config = *platform_config
        .into_iter()
        .find_map(|cfg| cfg.downcast::<Config>().ok())
        .unwrap_or_default();

    launch_virtual_dom(virtual_dom, platform_config)
}
