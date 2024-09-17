use std::any::Any;

use crate::FileEngine;
use futures_channel::oneshot;
use js_sys::Uint8Array;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{File, FileList, FileReader};

/// A file engine for the web platform
pub struct WebFileEngine {
    file_reader: FileReader,
    file_list: FileList,
}

impl WebFileEngine {
    /// Create a new file engine from a file list
    pub fn new(file_list: FileList) -> Option<Self> {
        Some(Self {
            file_list,
            file_reader: FileReader::new().ok()?,
        })
    }

    fn len(&self) -> usize {
        self.file_list.length() as usize
    }

    fn get(&self, index: usize) -> Option<File> {
        self.file_list.item(index as u32)
    }

    fn find(&self, name: &str) -> Option<File> {
        (0..self.len())
            .filter_map(|i| self.get(i))
            .find(|f| f.name() == name)
    }
}

#[async_trait::async_trait(?Send)]
impl FileEngine for WebFileEngine {
    fn files(&self) -> Vec<String> {
        (0..self.len())
            .filter_map(|i| self.get(i).map(|f| f.name()))
            .collect()
    }

    async fn file_size(&self, file: &str) -> Option<u64> {
        let file = self.find(file)?;
        Some(file.size() as u64)
    }

    // read a file to bytes
    async fn read_file(&self, file: &str) -> Option<Vec<u8>> {
        let file = self.find(file)?;

        let file_reader = self.file_reader.clone();
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

        self.file_reader
            .set_onload(Some(on_load.as_ref().unchecked_ref()));
        on_load.forget();
        self.file_reader.read_as_array_buffer(&file).ok()?;

        if let Ok(Ok(js_val)) = tx.await {
            let as_u8_arr = Uint8Array::new(&js_val);
            let as_u8_vec = as_u8_arr.to_vec();

            Some(as_u8_vec)
        } else {
            None
        }
    }

    // read a file to string
    async fn read_file_to_string(&self, file: &str) -> Option<String> {
        let file = self.find(file)?;

        let file_reader = self.file_reader.clone();
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

        self.file_reader
            .set_onload(Some(on_load.as_ref().unchecked_ref()));
        on_load.forget();
        self.file_reader.read_as_text(&file).ok()?;

        if let Ok(Ok(js_val)) = tx.await {
            js_val.as_string()
        } else {
            None
        }
    }

    async fn get_native_file(&self, file: &str) -> Option<Box<dyn Any>> {
        let file = self.find(file)?;
        Some(Box::new(file))
    }
}

/// Helper trait for WebFileEngine
#[async_trait::async_trait(?Send)]
pub trait WebFileEngineExt {
    /// returns web_sys::File
    async fn get_web_file(&self, file: &str) -> Option<web_sys::File>;
}

#[async_trait::async_trait(?Send)]
impl WebFileEngineExt for std::sync::Arc<dyn FileEngine> {
    async fn get_web_file(&self, file: &str) -> Option<web_sys::File> {
        let native_file = self.get_native_file(file).await?;
        let ret = native_file.downcast::<web_sys::File>().ok()?;
        Some(*ret)
    }
}
