use dioxus_html::{FileData, HasFileData};
use web_sys::FileReader;

use crate::{WebFileData, WebFileEngine};

use super::Synthetic;

impl HasFileData for Synthetic<web_sys::Event> {
    fn files(&self) -> Vec<FileData> {
        use wasm_bindgen::JsCast;
        let target = self.event.target();

        if let Some(target) = target
            .clone()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        {
            if let Some(file_list) = target.files() {
                return WebFileEngine::new(file_list).to_files();
            }
        }

        if let Some(target) = target.and_then(|t| t.dyn_into::<web_sys::DragEvent>().ok()) {
            if let Some(data_transfer) = target.data_transfer() {
                if let Some(file_list) = data_transfer.files() {
                    return WebFileEngine::new(file_list).to_files();
                } else {
                    let items = data_transfer.items();
                    let mut files = vec![];
                    for i in 0..items.length() {
                        if let Some(item) = items.get(i) {
                            if item.kind() == "file" {
                                if let Ok(Some(file)) = item.get_as_file() {
                                    let web_data =
                                        WebFileData::new(file, FileReader::new().unwrap());
                                    files.push(FileData::new(web_data));
                                }
                            }
                        }
                    }
                    return files;
                }
            }
        }

        vec![]
    }
}
