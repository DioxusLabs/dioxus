use anyhow::Result;
use image::load_from_memory;
use image::GenericImageView;
use image::ImageReader;
use std::path::Path;

/// Pre-decoded RGBA bytes of the bundled fallback icon.
const FALLBACK_ICON_RGBA: &[u8] = include_bytes!("./assets/default_icon.bin");
const FALLBACK_ICON_WIDTH: u32 = 460;
const FALLBACK_ICON_HEIGHT: u32 = 460;

/// Trait that creates icons for various types
pub trait DioxusIconTrait {
    fn get_icon() -> Result<Self>
    where
        Self: Sized;
    fn from_memory(value: &[u8]) -> Result<Self>
    where
        Self: Sized;
    fn path<P: AsRef<Path>>(path: P, size: Option<(u32, u32)>) -> Result<Self>
    where
        Self: Sized;
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use crate::trayicon::DioxusTrayIcon;

fn load_image_from_memory(value: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    let img = load_from_memory(value)?;
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();
    Ok((rgba.to_vec(), width, height))
}

fn load_image_from_path<P: AsRef<Path>>(path: P) -> Result<(Vec<u8>, u32, u32)> {
    let img = ImageReader::open(path)?.decode()?;
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();
    Ok((rgba.to_vec(), width, height))
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
impl DioxusIconTrait for DioxusTrayIcon {
    fn get_icon() -> Result<Self>
    where
        Self: Sized,
    {
        #[cfg(target_os = "windows")]
        if let Ok(icon) = DioxusTrayIcon::from_resource(32512, None) {
            return Ok(icon);
        }
        DioxusTrayIcon::from_rgba(
            FALLBACK_ICON_RGBA.to_vec(),
            FALLBACK_ICON_WIDTH,
            FALLBACK_ICON_HEIGHT,
        )
        .map_err(Into::into)
    }

    fn from_memory(value: &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let (icon, width, height) = load_image_from_memory(value)?;
        DioxusTrayIcon::from_rgba(icon, width, height).map_err(Into::into)
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
    fn get_icon() -> Result<Self>
    where
        Self: Sized,
    {
        #[cfg(target_os = "windows")]
        if let Ok(icon) = DioxusMenuIcon::from_resource(32512, None) {
            return Ok(icon);
        }
        DioxusMenuIcon::from_rgba(
            FALLBACK_ICON_RGBA.to_vec(),
            FALLBACK_ICON_WIDTH,
            FALLBACK_ICON_HEIGHT,
        )
        .map_err(Into::into)
    }

    fn from_memory(value: &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let (icon, width, height) = load_image_from_memory(value)?;
        DioxusMenuIcon::from_rgba(icon, width, height).map_err(Into::into)
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
    fn get_icon() -> Result<Self>
    where
        Self: Sized,
    {
        #[cfg(target_os = "windows")]
        if let Ok(icon) = Icon::from_resource(32512, None) {
            return Ok(icon);
        }
        Icon::from_rgba(
            FALLBACK_ICON_RGBA.to_vec(),
            FALLBACK_ICON_WIDTH,
            FALLBACK_ICON_HEIGHT,
        )
        .map_err(Into::into)
    }

    fn from_memory(value: &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let (icon, width, height) = load_image_from_memory(value)?;
        Icon::from_rgba(icon, width, height).map_err(Into::into)
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

/// Provides the default icon of the app.
///
/// On Windows this prefers the icon embedded as resource id `IDI::APPLICATION`
/// (32512) by `dx`'s bundler, falling back to a generic Dioxus icon when the
/// resource is missing (e.g. when running with plain `cargo run`). On all
/// other platforms the bundled fallback icon is returned directly.
pub fn default_icon<T: DioxusIconTrait>() -> Result<T> {
    T::get_icon()
}

/// Helper function to load image from include_bytes!("image.png")
pub fn icon_from_memory<T: DioxusIconTrait>(value: &[u8]) -> Result<T> {
    T::from_memory(value)
}

/// Helper function to load image from path
pub fn icon_from_path<T: DioxusIconTrait, P: AsRef<Path>>(
    path: P,
    size: Option<(u32, u32)>,
) -> Result<T> {
    T::path(path, size)
}
