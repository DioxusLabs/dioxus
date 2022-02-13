use wry::application::event_loop::ControlFlow;
use wry::application::window::Fullscreen as WryFullscreen;

use crate::controller::DesktopController;

pub(crate) enum UserWindowEvent {
    Update,

    CloseWindow,
    DragWindow,
    FocusWindow,

    Visible(bool),
    Minimize(bool),
    Maximize(bool),
    MaximizeToggle,
    Resizable(bool),
    AlwaysOnTop(bool),
    Fullscreen(bool),

    CursorVisible(bool),
    CursorGrab(bool),

    SetTitle(String),
    SetDecorations(bool),

    DevTool,
}

use UserWindowEvent::*;

pub(super) fn handler(
    user_event: UserWindowEvent,
    desktop: &mut DesktopController,
    control_flow: &mut ControlFlow,
) {
    // currently dioxus-desktop supports a single window only,
    // so we can grab the only webview from the map;
    let webview = desktop.webviews.values().next().unwrap();
    let window = webview.window();

    match user_event {
        Update => desktop.try_load_ready_webviews(),
        CloseWindow => *control_flow = ControlFlow::Exit,
        DragWindow => {
            // if the drag_window has any errors, we don't do anything
            window.fullscreen().is_none().then(|| window.drag_window());
        }
        Visible(state) => window.set_visible(state),
        Minimize(state) => window.set_minimized(state),
        Maximize(state) => window.set_maximized(state),
        MaximizeToggle => window.set_maximized(!window.is_maximized()),
        Fullscreen(state) => {
            window.current_monitor().map(|handle| {
                window.set_fullscreen(state.then(|| WryFullscreen::Borderless(Some(handle))));
            });
        }
        FocusWindow => window.set_focus(),
        Resizable(state) => window.set_resizable(state),
        AlwaysOnTop(state) => window.set_always_on_top(state),

        CursorVisible(state) => window.set_cursor_visible(state),
        CursorGrab(state) => {
            let _ = window.set_cursor_grab(state);
        }

        SetTitle(content) => window.set_title(&content),
        SetDecorations(state) => window.set_decorations(state),

        DevTool => webview.devtool(),
    }
}
