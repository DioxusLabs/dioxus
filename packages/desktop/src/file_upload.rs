#![allow(unused)]

use dioxus_html::{
    geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint},
    input_data::{MouseButton, MouseButtonSet},
    native_bind::NativeFileEngine,
    point_interaction::{
        InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
    },
    prelude::{SerializedMouseData, SerializedPointInteraction},
    FileEngine, HasDragData, HasFileData, HasFormData, HasMouseData,
};

use serde::Deserialize;
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    str::FromStr,
    sync::Arc,
};
use wry::DragDropEvent;

#[derive(Debug, Deserialize)]
pub(crate) struct FileDialogRequest {
    #[serde(default)]
    accept: Option<String>,
    multiple: bool,
    directory: bool,
    pub event: String,
    pub target: usize,
    pub bubbles: bool,
}

#[allow(unused)]
impl FileDialogRequest {
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    pub(crate) fn get_file_event(&self) -> Vec<PathBuf> {
        vec![]
    }

    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    pub(crate) fn get_file_event(&self) -> Vec<PathBuf> {
        fn get_file_event_for_folder(
            request: &FileDialogRequest,
            dialog: rfd::FileDialog,
        ) -> Vec<PathBuf> {
            if request.multiple {
                dialog.pick_folders().into_iter().flatten().collect()
            } else {
                dialog.pick_folder().into_iter().collect()
            }
        }

        fn get_file_event_for_file(
            request: &FileDialogRequest,
            mut dialog: rfd::FileDialog,
        ) -> Vec<PathBuf> {
            let filters: Vec<_> = request
                .accept
                .as_deref()
                .unwrap_or_default()
                .split(',')
                .filter_map(|s| Filters::from_str(s).ok())
                .collect();

            let file_extensions: Vec<_> = filters
                .iter()
                .flat_map(|f| f.as_extensions().into_iter())
                .collect();

            dialog = dialog.add_filter("name", file_extensions.as_slice());

            let files: Vec<_> = if request.multiple {
                dialog.pick_files().into_iter().flatten().collect()
            } else {
                dialog.pick_file().into_iter().collect()
            };

            files
        }

        let dialog = rfd::FileDialog::new();

        if self.directory {
            get_file_event_for_folder(self, dialog)
        } else {
            get_file_event_for_file(self, dialog)
        }
    }
}

enum Filters {
    Extension(String),
    Mime(String),
    Audio,
    Video,
    Image,
}

impl Filters {
    fn as_extensions(&self) -> Vec<&str> {
        match self {
            Filters::Extension(extension) => vec![extension.as_str()],
            Filters::Mime(_) => vec![],
            Filters::Audio => vec!["mp3", "wav", "ogg"],
            Filters::Video => vec!["mp4", "webm"],
            Filters::Image => vec!["png", "jpg", "jpeg", "gif", "webp"],
        }
    }
}

impl FromStr for Filters {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(extension) = s.strip_prefix('.') {
            Ok(Filters::Extension(extension.to_string()))
        } else {
            match s {
                "audio/*" => Ok(Filters::Audio),
                "video/*" => Ok(Filters::Video),
                "image/*" => Ok(Filters::Image),
                _ => Ok(Filters::Mime(s.to_string())),
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct DesktopFileUploadForm {
    pub files: Arc<NativeFileEngine>,
}

impl HasFileData for DesktopFileUploadForm {
    fn files(&self) -> Option<Arc<dyn FileEngine>> {
        Some(self.files.clone())
    }
}

impl HasFormData for DesktopFileUploadForm {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Default, Clone)]
pub struct NativeFileHover {
    event: Rc<RefCell<Option<DragDropEvent>>>,
}
impl NativeFileHover {
    pub fn set(&self, event: DragDropEvent) {
        self.event.borrow_mut().replace(event);
    }

    pub fn current(&self) -> Option<DragDropEvent> {
        self.event.borrow_mut().clone()
    }
}

#[derive(Clone)]
pub(crate) struct DesktopFileDragEvent {
    pub mouse: SerializedPointInteraction,
    pub files: Arc<NativeFileEngine>,
}

impl HasFileData for DesktopFileDragEvent {
    fn files(&self) -> Option<Arc<dyn FileEngine>> {
        Some(self.files.clone())
    }
}

impl HasDragData for DesktopFileDragEvent {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasMouseData for DesktopFileDragEvent {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl InteractionLocation for DesktopFileDragEvent {
    fn client_coordinates(&self) -> ClientPoint {
        self.mouse.client_coordinates()
    }

    fn page_coordinates(&self) -> PagePoint {
        self.mouse.page_coordinates()
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        self.mouse.screen_coordinates()
    }
}

impl InteractionElementOffset for DesktopFileDragEvent {
    fn element_coordinates(&self) -> ElementPoint {
        self.mouse.element_coordinates()
    }

    fn coordinates(&self) -> Coordinates {
        self.mouse.coordinates()
    }
}

impl ModifiersInteraction for DesktopFileDragEvent {
    fn modifiers(&self) -> dioxus_html::prelude::Modifiers {
        self.mouse.modifiers()
    }
}

impl PointerInteraction for DesktopFileDragEvent {
    fn held_buttons(&self) -> MouseButtonSet {
        self.mouse.held_buttons()
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        self.mouse.trigger_button()
    }
}
