use std::{path::PathBuf, str::FromStr};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct FileDiologRequest {
    accept: String,
    multiple: bool,
    pub event: String,
    pub target: usize,
    pub bubbles: bool,
}

pub(crate) fn get_file_event(request: &FileDiologRequest) -> Vec<PathBuf> {
    let mut dialog = rfd::FileDialog::new();

    let filters: Vec<_> = request
        .accept
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
