#![allow(unused)]

use serde::Deserialize;
use std::{path::PathBuf, str::FromStr};

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
