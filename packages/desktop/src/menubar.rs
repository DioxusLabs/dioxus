use std::any::Any;

use tao::window::Window;

#[allow(unused)]
pub fn build_menu(window: &Window, default_menu_bar: bool) -> Option<Box<dyn Any>> {
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        return Some(Box::new(impl_::build_menu_bar(default_menu_bar, window)) as Box<dyn Any>);
    }

    None
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod impl_ {
    use super::*;
    use muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};

    /// Builds a standard menu bar depending on the users platform. It may be used as a starting point
    /// to further customize the menu bar and pass it to a [`WindowBuilder`](tao::window::WindowBuilder).
    /// > Note: The default menu bar enables macOS shortcuts like cut/copy/paste.
    /// > The menu bar differs per platform because of constraints introduced
    /// > by [`MenuItem`](tao::menu::MenuItem).
    #[allow(unused)]
    pub fn build_menu_bar(default: bool, window: &Window) -> Menu {
        let menu = Menu::new();

        #[cfg(target_os = "windows")]
        {
            use tao::platform::windows::WindowExtWindows;
            menu.init_for_hwnd(window.hwnd());
        }

        #[cfg(target_os = "linux")]
        {
            use tao::platform::unix::WindowExtUnix;
            menu.init_for_gtk_window(window.gtk_window(), window.default_vbox())
                .unwrap();
        }

        #[cfg(target_os = "macos")]
        {
            use tao::platform::macos::WindowExtMacOS;
            menu.init_for_nsapp();
        }

        if default {
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

            let edit_menu = Submenu::new("Edit", true);
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

            let help_menu = Submenu::new("Help", true);
            help_menu
                .append_items(&[&MenuItem::new("Toggle Developer Tools", true, None)])
                .unwrap();

            menu.append_items(&[&window_menu, &edit_menu, &help_menu])
                .unwrap();

            #[cfg(target_os = "macos")]
            {
                window_menu.set_as_windows_menu_for_nsapp();
                help_menu.set_as_help_menu_for_nsapp();
            }
        }

        menu
    }
}
