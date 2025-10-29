use anyhow::Result;
use image::load_from_memory;
use image::GenericImageView;
use image::ImageReader;
use std::path::Path;

pub trait DefaultIcon {
    fn get_icon() -> Self
    where
        Self: Sized;
    fn from_memory(value: &[u8]) -> Self
    where
        Self: Sized;
    fn path<P: AsRef<Path>>(path: P, size: Option<(u32, u32)>) -> Result<Self>
    where
        Self: Sized;
}

// TODO this should probably just be an assets path and then loaded with from_path OR include_bytes and image crate
// preferably it would load from the bundle icon for every platform not just windows
#[cfg(any(debug_assertions, not(target_os = "windows")))]
static DEFAULT_ICON: &[u8] = include_bytes!(env!("DIOXUS_APP_ICON"));

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use crate::trayicon::DioxusTrayIcon;

fn load_image_from_memory(value: &[u8]) -> (Vec<u8>, u32, u32) {
    let img = load_from_memory(value).expect("MISSING DEFAULT ICON");
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();
    (rgba.to_vec(), width, height)
}

fn load_image_from_path<P: AsRef<Path>>(path: P) -> Result<(Vec<u8>, u32, u32)> {
    let img = ImageReader::open(path)?.decode()?;
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();
    Ok((rgba.to_vec(), width, height))
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
impl DefaultIcon for DioxusTrayIcon {
    fn get_icon() -> Self
    where
        Self: Sized,
    {
        #[cfg(any(debug_assertions, target_os = "linux", target_os = "macos"))]
        {
            let (img, width, height) = load_image_from_memory(DEFAULT_ICON);
            DioxusTrayIcon::from_rgba(img, width, height).expect("image parse failed")
        }
        #[cfg(all(not(debug_assertions), target_os = "windows"))]
        DioxusTrayIcon::from_resource(32512, None).expect("image parse failed")
    }

    fn from_memory(value: &[u8]) -> Self
    where
        Self: Sized,
    {
        let (icon, width, height) = load_image_from_memory(value);
        DioxusTrayIcon::from_rgba(icon, width, height).expect("image parse failed")
    }

    fn path<P: AsRef<Path>>(path: P, size: Option<(u32, u32)>) -> Result<Self>
    where
        Self: Sized,
    {
        let (img, width, height) = load_image_from_path(path)?;
        if let Some((width, height)) = size {
            Ok(DioxusTrayIcon::from_rgba(img, width, height)?)
        } else {
            Ok(DioxusTrayIcon::from_rgba(img, width, height)?)
        }
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
        #[cfg(any(debug_assertions, not(target_os = "windows")))]
        {
            let (img, width, height) = load_image_from_memory(DEFAULT_ICON);
            DioxusMenuIcon::from_rgba(img, width, height).expect("image parse failed")
        }
        #[cfg(all(not(debug_assertions), target_os = "windows"))]
        DioxusMenuIcon::from_resource(32512, None).expect("image parse failed")
    }

    fn from_memory(value: &[u8]) -> Self
    where
        Self: Sized,
    {
        let (icon, width, height) = load_image_from_memory(value);
        DioxusMenuIcon::from_rgba(icon, width, height).expect("image parse failed")
    }

    fn path<P: AsRef<Path>>(path: P, size: Option<(u32, u32)>) -> Result<Self>
    where
        Self: Sized,
    {
        let (img, width, height) = load_image_from_path(path)?;
        if let Some((width, height)) = size {
            Ok(DioxusMenuIcon::from_rgba(img, width, height)?)
        } else {
            Ok(DioxusMenuIcon::from_rgba(img, width, height)?)
        }
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
        #[cfg(any(debug_assertions, not(target_os = "windows")))]
        {
            let (img, width, height) = load_image_from_memory(DEFAULT_ICON);
            Icon::from_rgba(img, width, height).expect("image parse failed")
        }
        #[cfg(all(not(debug_assertions), target_os = "windows"))]
        Icon::from_resource(32512, None).expect("image parse failed")
    }

    fn from_memory(value: &[u8]) -> Self
    where
        Self: Sized,
    {
        let (icon, width, height) = load_image_from_memory(value);
        Icon::from_rgba(icon, width, height).expect("image parse failed")
    }

    fn path<P: AsRef<Path>>(path: P, size: Option<(u32, u32)>) -> Result<Self>
    where
        Self: Sized,
    {
        let (img, width, height) = load_image_from_path(path)?;
        if let Some((width, height)) = size {
            Ok(Icon::from_rgba(img, width, height)?)
        } else {
            Ok(Icon::from_rgba(img, width, height)?)
        }
    }
}

/// Provides the default icon of the app
/// NOTE only implemented for windows --release, otherwise it will be just a classic dioxus icon
pub fn default_icon<T: DefaultIcon>() -> T {
    T::get_icon()
}

pub fn icon_from_memory<T: DefaultIcon>(value: &[u8]) -> T {
    T::from_memory(value)
}

pub fn icon_from_path<T: DefaultIcon, P: AsRef<Path>>(
    path: P,
    size: Option<(u32, u32)>,
) -> Result<T> {
    T::path(path, size)
}
