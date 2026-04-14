#![allow(unused)]

use std::{any::Any, collections::HashMap};

#[cfg(feature = "tokio_runtime")]
use tokio::{fs::File, io::AsyncReadExt};

use dioxus_html::{
    geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint},
    input_data::{MouseButton, MouseButtonSet},
    point_interaction::{
        InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
    },
    FileData, FormValue, HasDataTransferData, HasDragData, HasFileData, HasFormData, HasMouseData,
    NativeFileData, SerializedDataTransfer, SerializedFormData, SerializedFormObject,
    SerializedMouseData, SerializedPointInteraction,
};

use serde::{Deserialize, Serialize};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    str::FromStr,
    sync::Arc,
};
use wry::DragDropEvent;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct FileDialogRequest {
    #[serde(default)]
    accept: Option<String>,
    multiple: bool,
    directory: bool,
    pub event: String,
    pub target: usize,
    pub bubbles: bool,
    pub target_name: String,
    pub values: Vec<SerializedFormObject>,
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

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    pub(crate) async fn get_file_event_async(&self) -> Vec<PathBuf> {
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
    pub(crate) fn get_file_event_sync(&self) -> Vec<PathBuf> {
        let dialog = rfd::FileDialog::new();
        if self.directory {
            self.get_file_event_for_folder(dialog)
        } else {
            self.get_file_event_for_file(dialog)
        }
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
    pub(crate) async fn get_file_event_async(&self) -> Vec<PathBuf> {
        let mut dialog = rfd::AsyncFileDialog::new();

        if self.directory {
            if self.multiple {
                dialog
                    .pick_folders()
                    .await
                    .into_iter()
                    .flatten()
                    .map(|f| f.path().to_path_buf())
                    .collect()
            } else {
                dialog
                    .pick_folder()
                    .await
                    .into_iter()
                    .map(|f| f.path().to_path_buf())
                    .collect()
            }
        } else {
            let filters: Vec<_> = self
                .accept
                .as_deref()
                .unwrap_or(".*")
                .split(',')
                .filter_map(|s| Filters::from_str(s.trim()).ok())
                .collect();

            let file_extensions: Vec<_> = filters
                .iter()
                .flat_map(|f| f.as_extensions().into_iter())
                .collect();

            let filter_name = file_extensions
                .iter()
                .map(|extension| format!("*.{extension}"))
                .collect::<Vec<_>>()
                .join(", ");

            dialog = dialog.add_filter(filter_name, file_extensions.as_slice());

            let files: Vec<_> = if self.multiple {
                dialog
                    .pick_files()
                    .await
                    .into_iter()
                    .flatten()
                    .map(|f| f.path().to_path_buf())
                    .collect()
            } else {
                dialog
                    .pick_file()
                    .await
                    .into_iter()
                    .map(|f| f.path().to_path_buf())
                    .collect()
            };

            files
        }
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
    fn get_file_event_for_file(&self, mut dialog: rfd::FileDialog) -> Vec<PathBuf> {
        let filters: Vec<_> = self
            .accept
            .as_deref()
            .unwrap_or(".*")
            .split(',')
            .filter_map(|s| Filters::from_str(s.trim()).ok())
            .collect();

        let file_extensions: Vec<_> = filters
            .iter()
            .flat_map(|f| f.as_extensions().into_iter())
            .collect();

        let filter_name = file_extensions
            .iter()
            .map(|extension| format!("*.{extension}"))
            .collect::<Vec<_>>()
            .join(", ");

        dialog = dialog.add_filter(filter_name, file_extensions.as_slice());

        let files: Vec<_> = if self.multiple {
            dialog.pick_files().into_iter().flatten().collect()
        } else {
            dialog.pick_file().into_iter().collect()
        };

        files
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
    fn get_file_event_for_folder(&self, dialog: rfd::FileDialog) -> Vec<PathBuf> {
        if self.multiple {
            dialog.pick_folders().into_iter().flatten().collect()
        } else {
            dialog.pick_folder().into_iter().collect()
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
pub(crate) struct DesktopFormData {
    pub value: String,
    pub valid: bool,
    pub values: Vec<(String, FormValue)>,
}

impl DesktopFormData {
    pub fn new(values: Vec<(String, FormValue)>) -> Self {
        Self {
            value: String::new(),
            valid: true,
            values,
        }
    }
}

impl HasFileData for DesktopFormData {
    fn files(&self) -> Vec<FileData> {
        self.values
            .iter()
            .filter_map(|(_, v)| {
                if let FormValue::File(Some(f)) = v {
                    Some(f.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

impl HasFormData for DesktopFormData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn value(&self) -> String {
        self.value.clone()
    }

    fn valid(&self) -> bool {
        self.valid
    }

    fn values(&self) -> Vec<(String, FormValue)> {
        self.values.clone()
    }
}

#[derive(Default, Clone)]
pub struct NativeFileHover {
    event: Rc<RefCell<Option<DragDropEvent>>>,
    paths: Rc<RefCell<Vec<PathBuf>>>,
}
impl NativeFileHover {
    pub fn set(&self, event: DragDropEvent) {
        match event {
            DragDropEvent::Enter { ref paths, .. } => self.paths.borrow_mut().clone_from(paths),
            DragDropEvent::Drop { ref paths, .. } => self.paths.borrow_mut().clone_from(paths),
            _ => {}
        }
        self.event.borrow_mut().replace(event);
    }

    pub fn current(&self) -> Option<DragDropEvent> {
        self.event.borrow_mut().clone()
    }

    pub fn current_paths(&self) -> Vec<PathBuf> {
        self.paths.borrow_mut().clone()
    }
}

#[derive(Clone)]
pub(crate) struct DesktopFileDragEvent {
    pub mouse: SerializedPointInteraction,
    pub data_transfer: SerializedDataTransfer,
    pub files: Vec<PathBuf>,
}

impl HasFileData for DesktopFileDragEvent {
    fn files(&self) -> Vec<FileData> {
        self.files
            .iter()
            .cloned()
            .map(|f| FileData::new(DesktopFileData(f)))
            .collect()
    }
}

impl HasDataTransferData for DesktopFileDragEvent {
    fn data_transfer(&self) -> dioxus_html::DataTransfer {
        dioxus_html::DataTransfer::new(self.data_transfer.clone())
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
    fn modifiers(&self) -> dioxus_html::Modifiers {
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

#[derive(Clone)]
pub struct DesktopFileData(pub(crate) PathBuf);

impl NativeFileData for DesktopFileData {
    fn name(&self) -> String {
        self.0.file_name().unwrap().to_string_lossy().into_owned()
    }

    fn size(&self) -> u64 {
        std::fs::metadata(&self.0).map(|m| m.len()).unwrap_or(0)
    }

    fn last_modified(&self) -> u64 {
        std::fs::metadata(&self.0)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    }

    fn read_bytes(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<bytes::Bytes, dioxus_core::CapturedError>>
                + 'static,
        >,
    > {
        let path = self.0.clone();
        Box::pin(async move { Ok(bytes::Bytes::from(std::fs::read(&path)?)) })
    }

    fn read_string(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, dioxus_core::CapturedError>> + 'static>,
    > {
        let path = self.0.clone();
        Box::pin(async move { Ok(std::fs::read_to_string(&path)?) })
    }

    fn inner(&self) -> &dyn std::any::Any {
        &self.0
    }

    fn path(&self) -> PathBuf {
        self.0.clone()
    }

    fn byte_stream(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn futures_util::Stream<Item = Result<bytes::Bytes, dioxus_core::CapturedError>>
                + 'static
                + Send,
        >,
    > {
        let path = self.0.clone();
        Box::pin(futures_util::stream::once(async move {
            Ok(bytes::Bytes::from(std::fs::read(&path)?))
        }))
    }

    fn content_type(&self) -> Option<String> {
        Some(
            dioxus_asset_resolver::native::get_mime_from_ext(
                self.0.extension().and_then(|ext| ext.to_str()),
            )
            .to_string(),
        )
    }
}

pub struct DesktopDataTransfer {}
