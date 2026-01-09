use dioxus_core::AnyhowContext;
use dioxus_html::{bytes::Bytes, FileData, NativeFileData};
use futures_channel::oneshot;
use js_sys::Uint8Array;
use send_wrapper::SendWrapper;
use std::{pin::Pin, prelude::rust_2024::Future};
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{File, FileList, FileReader};

/// A file representation for the web platform
#[derive(Clone)]
pub struct WebFileData {
    file: File,
    reader: FileReader,
}

unsafe impl Send for WebFileData {}
unsafe impl Sync for WebFileData {}

impl WebFileData {
    /// Create a new WebFileData from a web_sys::File
    pub fn new(file: File, reader: FileReader) -> Self {
        Self { file, reader }
    }
}

impl NativeFileData for WebFileData {
    fn name(&self) -> String {
        self.file.name()
    }

    fn size(&self) -> u64 {
        self.file.size() as u64
    }

    fn last_modified(&self) -> u64 {
        self.file.last_modified() as u64
    }

    fn read_bytes(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Bytes, dioxus_core::CapturedError>> + 'static>> {
        let file_reader = self.reader.clone();
        let file_reader_ = self.reader.clone();
        let file = self.file.clone();
        Box::pin(async move {
            let (rx, tx) = oneshot::channel();
            let on_load: Closure<dyn FnMut()> = Closure::new({
                let mut rx = Some(rx);
                move || {
                    let result = file_reader.result();
                    let _ = rx
                        .take()
                        .expect("multiple files read without refreshing the channel")
                        .send(result);
                }
            });

            file_reader_.set_onload(Some(on_load.as_ref().unchecked_ref()));
            on_load.forget();
            file_reader_
                .read_as_array_buffer(&file)
                .ok()
                .context("Failed to read file")?;

            let js_val = tx.await?.ok().context("Failed to read file")?;
            let as_u8_arr = Uint8Array::new(&js_val);
            let as_u8_vec = as_u8_arr.to_vec().into();
            Ok(as_u8_vec)
        })
    }

    fn read_string(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<String, dioxus_core::CapturedError>> + 'static>> {
        let file_reader = self.reader.clone();
        let file_reader_ = self.reader.clone();
        let file = self.file.clone();
        Box::pin(async move {
            let (rx, tx) = oneshot::channel();
            let on_load: Closure<dyn FnMut()> = Closure::new({
                let mut rx = Some(rx);
                move || {
                    let result = file_reader.result();
                    let _ = rx
                        .take()
                        .expect("multiple files read without refreshing the channel")
                        .send(result);
                }
            });

            file_reader_.set_onload(Some(on_load.as_ref().unchecked_ref()));
            on_load.forget();
            file_reader_
                .read_as_text(&file)
                .ok()
                .context("Failed to read file")?;

            let js_val = tx.await?.ok().context("Failed to read file")?;
            let as_string = js_val.as_string().context("Failed to read file")?;
            Ok(as_string)
        })
    }

    /// we'd like to use `blob` to readable stream here, but we cannot.
    ///
    /// We just read the entire file into memory and return it as a single chunk.
    /// This is not super great, especially given the wasm <-> js boundary duplication cost.
    ///
    /// For more efficient streaming of byte data, consider using the dedicated FileStream type which
    /// goes directly from `File` to fetch request body without going through Rust.
    ///
    /// We should maybe update these APIs to use our own custom `ByteBuffer` type to avoid going through `Vec<u8>`?
    fn byte_stream(
        &self,
    ) -> Pin<
        Box<
            dyn futures_util::Stream<Item = Result<Bytes, dioxus_core::CapturedError>>
                + 'static
                + Send,
        >,
    > {
        let file = self.file.dyn_ref::<web_sys::Blob>().unwrap().clone();
        Box::pin(SendWrapper::new(futures_util::stream::once(async move {
            let array_buff = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
                .await
                .unwrap();
            let as_uint_array = array_buff.dyn_into::<Uint8Array>().unwrap();
            Ok(as_uint_array.to_vec().into())
        })))
    }

    fn inner(&self) -> &dyn std::any::Any {
        &self.file
    }

    fn path(&self) -> std::path::PathBuf {
        let key = wasm_bindgen::JsValue::from_str("webkitRelativePath");

        if let Ok(value) = js_sys::Reflect::get(&self.file, &key) {
            if let Some(path_str) = value.as_string() {
                if !path_str.is_empty() {
                    return std::path::PathBuf::from(path_str);
                }
            }
        }

        std::path::PathBuf::from(self.file.name())
    }

    fn content_type(&self) -> Option<String> {
        let type_ = self.file.type_();
        if type_.is_empty() {
            None
        } else {
            Some(type_)
        }
    }
}

/// A file engine for the web platform
#[derive(Clone)]
pub(crate) struct WebFileEngine {
    file_list: FileList,
}

impl WebFileEngine {
    /// Create a new file engine from a file list
    pub fn new(file_list: FileList) -> Self {
        Self { file_list }
    }

    fn len(&self) -> usize {
        self.file_list.length() as usize
    }

    fn get(&self, index: usize) -> Option<File> {
        self.file_list.item(index as u32)
    }

    pub fn to_files(&self) -> Vec<FileData> {
        (0..self.len())
            .filter_map(|i| self.get(i))
            .map(|file| {
                FileData::new(WebFileData {
                    file,
                    reader: FileReader::new().unwrap(),
                })
            })
            .collect()
    }
}

/// Helper trait for extracting the underlying `web_sys::File` from a `FileData`
pub trait WebFileExt {
    /// returns web_sys::File
    fn get_web_file(&self) -> Option<web_sys::File>;
}

impl WebFileExt for FileData {
    fn get_web_file(&self) -> Option<web_sys::File> {
        self.inner().downcast_ref::<web_sys::File>().cloned()
    }
}
