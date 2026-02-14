use anyhow::Result;
use image::load_from_memory;
use image::GenericImageView;
use image::ImageReader;
use std::path::Path;

/// Trait that creates icons for various types
pub trait DioxusIconTrait {
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

// preferably this would have platform specific implementations, not just for windows
#[cfg(not(target_os = "windows"))]
static DEFAULT_ICON: &[u8] = include_bytes!("./assets/default_icon.png");

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
impl DioxusIconTrait for DioxusTrayIcon {
    fn get_icon() -> Self
    where
        Self: Sized,
    {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let (img, width, height) = load_image_from_memory(DEFAULT_ICON);
            DioxusTrayIcon::from_rgba(img, width, height).expect("image parse failed")
        }
        #[cfg(target_os = "windows")]
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
impl DioxusIconTrait for DioxusMenuIcon {
    fn get_icon() -> Self
    where
        Self: Sized,
    {
        #[cfg(not(target_os = "windows"))]
        {
            let (img, width, height) = load_image_from_memory(DEFAULT_ICON);
            DioxusMenuIcon::from_rgba(img, width, height).expect("image parse failed")
        }
        #[cfg(target_os = "windows")]
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

#[cfg(target_os = "windows")]
use tao::platform::windows::IconExtWindows;

impl DioxusIconTrait for Icon {
    fn get_icon() -> Self
    where
        Self: Sized,
    {
        #[cfg(not(target_os = "windows"))]
        {
            let (img, width, height) = load_image_from_memory(DEFAULT_ICON);
            Icon::from_rgba(img, width, height).expect("image parse failed")
        }
        #[cfg(target_os = "windows")]
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
pub fn default_icon<T: DioxusIconTrait>() -> T {
    T::get_icon()
}

/// Helper function to load image from include_bytes!("image.png")
pub fn icon_from_memory<T: DioxusIconTrait>(value: &[u8]) -> T {
    T::from_memory(value)
}

/// Helper function to load image from path
pub fn icon_from_path<T: DioxusIconTrait, P: AsRef<Path>>(
    path: P,
    size: Option<(u32, u32)>,
) -> Result<T> {
    T::path(path, size)
}
