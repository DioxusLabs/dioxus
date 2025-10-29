// TODO only implemented for windows --release, otherwise it will provide dioxus icon instead of user defined icon, needs implementation for other platforms
pub trait DefaultIcon {
    fn get_icon() -> Self
    where
        Self: Sized;
}

// TODO this should probably just be an assets path and then loaded with from_path OR include_bytes and image crate
// preferably it would load from the bundle icon for every platform not just windows
#[cfg(any(debug_assertions, not(target_os = "windows")))]
static ICON: &[u8] = include_bytes!(env!("DIOXUS_APP_ICON"));

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use crate::trayicon::DioxusTrayIcon;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
impl DefaultIcon for DioxusTrayIcon {
    fn get_icon() -> Self
    where
        Self: Sized,
    {
        #[cfg(any(debug_assertions, target_os = "linux", target_os = "macos"))]
        let default = DioxusTrayIcon::from_rgba(ICON.to_vec(), 460, 460);
        #[cfg(all(not(debug_assertions), target_os = "windows"))]
        let default = DioxusTrayIcon::from_resource(32512, None);

        default.expect("image parse failed")
    }
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
use crate::menubar::DioxusMenuIcon;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
impl DefaultIcon for DioxusMenuIcon {
    fn get_icon() -> Self
    where
        Self: Sized,
    {
        #[cfg(debug_assertions)]
        let default = DioxusMenuIcon::from_rgba(ICON.to_vec(), 460, 460);
        #[cfg(all(not(debug_assertions), target_os = "windows"))]
        let default = DioxusMenuIcon::from_resource(32512, None);

        default.expect("image parse failed")
    }
}

use tao::window::Icon;

#[cfg(all(not(debug_assertions), target_os = "windows"))]
use tao::platform::windows::IconExtWindows;

impl DefaultIcon for Icon {
    fn get_icon() -> Self
    where
        Self: Sized,
    {
        #[cfg(debug_assertions)]
        let default = Icon::from_rgba(ICON.to_vec(), 460, 460);

        #[cfg(all(not(debug_assertions), target_os = "windows"))]
        let default = Icon::from_resource(32512, None);

        default.expect("image parse failed")
    }
}

/// Provides the default icon of the app
/// NOTE only implemented for windows --release, otherwise it will be just a classic dioxus icon
pub fn default_icon<T: DefaultIcon>() -> T {
    T::get_icon()
}
