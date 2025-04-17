// TODO only implemented for windows, needs implementation for other platforms

pub trait DefaultIcon {
    fn get_icon() -> Self
    where
        Self: Sized;
}

#[cfg(not(target_os = "windows"))]
static ICON: &[u8] = include_bytes!("./assets/default_icon.bin");

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use crate::trayicon::DioxusTrayIcon;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
impl DefaultIcon for DioxusTrayIcon {
    fn get_icon() -> Self
    where
        Self: Sized,
    {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let default = DioxusTrayIcon::from_rgba(ICON.to_vec(), 460, 460);
        #[cfg(target_os = "windows")]
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
        #[cfg(not(any(target_os = "ios", target_os = "android", target_os = "windows")))]
        let default = DioxusMenuIcon::from_rgba(ICON.to_vec(), 460, 460);
        #[cfg(target_os = "windows")]
        let default = DioxusMenuIcon::from_resource(32512, None);

        default.expect("image parse failed")
    }
}

use tao::window::Icon;

#[cfg(target_os = "windows")]
use tao::platform::windows::IconExtWindows;

impl DefaultIcon for Icon {
    fn get_icon() -> Self
    where
        Self: Sized,
    {
        #[cfg(not(target_os = "windows"))]
        let default = Icon::from_rgba(ICON.to_vec(), 460, 460);

        #[cfg(target_os = "windows")]
        let default = Icon::from_resource(32512, None);

        default.expect("image parse failed")
    }
}

/// Provides the default icon of the app
pub fn default_icon<T: DefaultIcon>() -> T {
    T::get_icon()
}
