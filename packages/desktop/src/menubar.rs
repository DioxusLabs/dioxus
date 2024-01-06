use tao::window::Window;

#[allow(unused)]
pub fn build_menu(window: &Window, default_menu_bar: bool) {
    if default_menu_bar {
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        impl_::build_menu_bar(impl_::build_default_menu_bar(), window)
    }
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod impl_ {
    use super::*;
    use muda::{Menu, PredefinedMenuItem, Submenu};

    #[allow(unused)]
    pub fn build_menu_bar(menu: Menu, window: &Window) {
        #[cfg(target_os = "windows")]
        use tao::platform::windows::WindowExtWindows;

        #[cfg(target_os = "windows")]
        menu.init_for_hwnd(window.hwnd());

        // #[cfg(target_os = "linux")]
        // {
        // use tao::platform::unix::WindowExtUnix;
        // menu.init_for_gtk_window(window, None);
        // menu.init_for_gtk_window(window, Some(&ertical_gtk_box));
        // }

        #[cfg(target_os = "macos")]
        menu.init_for_nsapp();
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
}
