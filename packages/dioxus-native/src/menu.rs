use winit::window::Window;

/// Initialize the default menu bar.
pub fn init_menu(window: &Window) -> muda::Menu {
    use muda::{AboutMetadata, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};

    let menu = Menu::new();

    // Build the about section
    let about = Submenu::new("About", true);
    about
        .append_items(&[
            &PredefinedMenuItem::about("Dioxus".into(), Option::from(AboutMetadata::default())),
            &MenuItem::with_id(MenuId::new("dev.show_layout"), "Show layout", true, None),
        ])
        .unwrap();
    menu.append(&about).unwrap();

    #[cfg(target_os = "windows")]
    {
        use winit::raw_window_handle::*;
        if let RawWindowHandle::Win32(handle) = window.window_handle().unwrap().as_raw() {
            menu.init_for_hwnd(handle.hwnd.get()).unwrap();
        }
    }

    // todo: menu on linux
    // #[cfg(target_os = "linux")]
    // {
    //     use winit::platform::unix::WindowExtUnix;
    //     menu.init_for_gtk_window(window.gtk_window(), window.default_vbox())
    //         .unwrap();
    // }

    #[cfg(target_os = "macos")]
    {
        menu.init_for_nsapp();
    }

    // Suppress unused variable warning on non-windows platforms
    let _ = window;

    menu
}
