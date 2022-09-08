use std::path::PathBuf;

use super::super::*;
use on::*;

pub type DragEvent = UiEvent<DragData>;

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct DragData {
    pub transfer: DataTransfer,
    pub mouse_data: MouseData,
}

impl DragData {
    pub fn files(&self) -> &[PathBuf] {
        &self.transfer.files
    }
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct DataTransfer {
    pub files: Vec<PathBuf>,
}

impl std::ops::Deref for DragData {
    type Target = MouseData;
    fn deref(&self) -> &Self::Target {
        &self.mouse_data
    }
}
