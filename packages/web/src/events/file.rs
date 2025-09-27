use dioxus_html::{FileData, HasFileData};

use super::Synthetic;

impl HasFileData for Synthetic<web_sys::Event> {
    fn files(&self) -> Vec<FileData> {
        #[cfg(feature = "file_engine")]
        {
            use wasm_bindgen::JsCast;
            self.event
                .dyn_ref()
                .and_then(|input: &web_sys::HtmlInputElement| {
                    input.files().and_then(crate::files::WebFileEngine::new)
                })
                .map(|engine| engine.to_files())
                .unwrap_or_default()
        }

        #[cfg(not(feature = "file_engine"))]
        {
            vec![]
        }
    }
}
