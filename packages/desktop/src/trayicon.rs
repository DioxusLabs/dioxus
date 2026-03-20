//! tray icon

use dioxus_core::{provide_context, try_consume_context, use_hook};

#[cfg(not(any(target_os = "ios", target_os = "android", target_env = "ohos")))]
pub use tray_icon::*;

/// tray icon menu type trait
#[cfg(not(any(target_os = "ios", target_os = "android", target_env = "ohos")))]
pub type DioxusTrayMenu = tray_icon::menu::Menu;
#[cfg(any(target_os = "ios", target_os = "android", target_env = "ohos"))]
pub type DioxusTrayMenu = ();

/// tray icon icon type trait
#[cfg(not(any(target_os = "ios", target_os = "android", target_env = "ohos")))]
pub type DioxusTrayIcon = tray_icon::Icon;
#[cfg(any(target_os = "ios", target_os = "android", target_env = "ohos"))]
pub type DioxusTrayIcon = ();

/// tray icon type trait
#[cfg(not(any(target_os = "ios", target_os = "android", target_env = "ohos")))]
pub type DioxusTray = tray_icon::TrayIcon;
#[cfg(any(target_os = "ios", target_os = "android", target_env = "ohos"))]
pub type DioxusTray = ();

/// initializes a tray icon
#[allow(unused)]
pub fn init_tray_icon(menu: DioxusTrayMenu, icon: Option<DioxusTrayIcon>) -> DioxusTray {
    #[cfg(all(
        any(target_os = "windows", target_os = "linux", target_os = "macos"),
        not(target_env = "ohos")
    ))]
    {
        let builder = tray_icon::TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_icon(match icon {
                Some(value) => value,
                None => tray_icon::Icon::from_rgba(
                    include_bytes!("./assets/default_icon.bin").to_vec(),
                    460,
                    460,
                )
                .expect("image parse failed"),
            });

        provide_context(builder.build().expect("tray icon builder failed"))
    }
}

/// Returns a default tray icon menu
pub fn default_tray_icon() -> DioxusTrayMenu {
    #[cfg(all(
        any(target_os = "windows", target_os = "linux", target_os = "macos"),
        not(target_env = "ohos")
    ))]
    {
        use tray_icon::menu::{Menu, PredefinedMenuItem};
        let tray_menu = Menu::new();
        tray_menu
            .append_items(&[&PredefinedMenuItem::quit(None)])
            .unwrap();
        tray_menu
    }
}

/// Provides a hook to the tray icon
#[cfg(all(
    any(target_os = "windows", target_os = "linux", target_os = "macos"),
    not(target_env = "ohos")
))]
pub fn use_tray_icon() -> Option<tray_icon::TrayIcon> {
    use_hook(try_consume_context)
}
