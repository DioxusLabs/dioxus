use crate::WebFileData;
use dioxus_html::{FileData, NativeDataTransfer};

/// A wrapper around the web_sys::DataTransfer to implement NativeDataTransfer
#[derive(Clone)]
pub struct WebDataTransfer {
    pub(crate) data: web_sys::DataTransfer,
}

impl WebDataTransfer {
    /// Create a new WebDataTransfer from a web_sys::DataTransfer
    pub fn new(data: web_sys::DataTransfer) -> Self {
        Self { data }
    }
}

unsafe impl Send for WebDataTransfer {}
unsafe impl Sync for WebDataTransfer {}

impl NativeDataTransfer for WebDataTransfer {
    fn get_data(&self, format: &str) -> Option<String> {
        self.data.get_data(format).ok()
    }
    fn set_data(&self, format: &str, data: &str) -> Result<(), String> {
        self.data.set_data(format, data).map_err(|e| {
            format!(
                "Failed to set data for format {format}: {:?}",
                e.as_string()
            )
        })
    }
    fn clear_data(&self, format: Option<&str>) -> Result<(), String> {
        match format {
            Some(f) => self.data.clear_data_with_format(f),
            None => self.data.clear_data(),
        }
        .map_err(|e| format!("{:?}", e))
    }
    fn effect_allowed(&self) -> String {
        self.data.effect_allowed()
    }
    fn set_effect_allowed(&self, effect: &str) {
        self.data.set_effect_allowed(effect);
    }
    fn drop_effect(&self) -> String {
        self.data.drop_effect()
    }
    fn set_drop_effect(&self, effect: &str) {
        self.data.set_drop_effect(effect);
    }

    fn files(&self) -> Vec<FileData> {
        let mut result = Vec::new();
        if let Some(file_list) = self.data.files() {
            for i in 0..file_list.length() {
                if let Some(file) = file_list.item(i) {
                    result.push(FileData::new(WebFileData::new(
                        file,
                        web_sys::FileReader::new().unwrap(),
                    )));
                }
            }
        }
        result
    }
}
