use dioxus_html::HasFileData;

use super::Synthetic;

impl HasFileData for Synthetic<web_sys::Event> {
    fn files(&self) -> Option<std::sync::Arc<dyn dioxus_html::FileEngine>> {
        #[cfg(feature = "file_engine")]
        {
            use wasm_bindgen::JsCast;

            let files = self
                .event
                .dyn_ref()
                .and_then(|input: &web_sys::HtmlInputElement| {
                    input.files().and_then(|files| {
                        #[allow(clippy::arc_with_non_send_sync)]
                        crate::file_engine::WebFileEngine::new(files).map(|f| {
                            std::sync::Arc::new(f) as std::sync::Arc<dyn dioxus_html::FileEngine>
                        })
                    })
                });

            files
        }
        #[cfg(not(feature = "file_engine"))]
        {
            None
        }
    }
}
