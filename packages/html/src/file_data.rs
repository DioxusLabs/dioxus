use std::{path::PathBuf, pin::Pin, prelude::rust_2024::Future};

use bytes::Bytes;
use futures_util::Stream;

#[derive(Clone)]
pub struct FileData {
    inner: std::sync::Arc<dyn NativeFileData>,
}

impl FileData {
    pub fn new(inner: impl NativeFileData + 'static) -> Self {
        Self {
            inner: std::sync::Arc::new(inner),
        }
    }

    pub fn content_type(&self) -> Option<String> {
        self.inner.content_type()
    }

    pub fn name(&self) -> String {
        self.inner.name()
    }

    pub fn size(&self) -> u64 {
        self.inner.size()
    }

    pub fn last_modified(&self) -> u64 {
        self.inner.last_modified()
    }

    pub async fn read_bytes(&self) -> Result<Bytes, dioxus_core::Error> {
        self.inner.read_bytes().await
    }

    pub async fn read_string(&self) -> Result<String, dioxus_core::Error> {
        self.inner.read_string().await
    }

    pub fn byte_stream(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, dioxus_core::Error>> + Send + 'static>> {
        self.inner.byte_stream()
    }

    pub fn inner(&self) -> &dyn std::any::Any {
        self.inner.inner()
    }

    pub fn pathbuf(&self) -> PathBuf {
        self.inner.path()
    }
}

pub trait NativeFileData: Send + Sync {
    fn name(&self) -> String;
    fn size(&self) -> u64;
    fn last_modified(&self) -> u64;
    fn path(&self) -> PathBuf;
    fn content_type(&self) -> Option<String>;
    fn read_bytes(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Bytes, dioxus_core::Error>> + 'static>>;
    fn byte_stream(
        &self,
    ) -> Pin<Box<dyn futures_util::Stream<Item = Result<Bytes, dioxus_core::Error>> + 'static + Send>>;
    fn read_string(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<String, dioxus_core::Error>> + 'static>>;
    fn inner(&self) -> &dyn std::any::Any;
}

impl std::fmt::Debug for FileData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileData")
            .field("name", &self.inner.name())
            .field("size", &self.inner.size())
            .field("last_modified", &self.inner.last_modified())
            .finish()
    }
}

impl std::cmp::PartialEq for FileData {
    fn eq(&self, other: &Self) -> bool {
        self.inner.name() == other.inner.name()
            && self.inner.size() == other.inner.size()
            && self.inner.last_modified() == other.inner.last_modified()
    }
}

pub trait HasFileData: std::any::Any {
    fn files(&self) -> Vec<FileData>;
}

/// A serializable representation of file data\
#[cfg(feature = "serialize")]
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedFileData {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub last_modified: u64,
    pub content_type: Option<String>,
    pub contents: bytes::Bytes,
}

#[cfg(feature = "serialize")]
impl NativeFileData for SerializedFileData {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn last_modified(&self) -> u64 {
        self.last_modified
    }

    fn read_bytes(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Bytes, dioxus_core::Error>> + 'static>> {
        let contents = self.contents.clone();
        Box::pin(async move { Ok(contents) })
    }

    fn read_string(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<String, dioxus_core::Error>> + 'static>> {
        let contents = self.contents.clone();
        Box::pin(async move { Ok(String::from_utf8(contents.to_vec())?) })
    }

    fn byte_stream(
        &self,
    ) -> Pin<Box<dyn futures_util::Stream<Item = Result<Bytes, dioxus_core::Error>> + 'static + Send>>
    {
        let contents = self.contents.clone();
        Box::pin(futures_util::stream::once(async move { Ok(contents) }))
    }

    fn inner(&self) -> &dyn std::any::Any {
        self
    }

    fn path(&self) -> PathBuf {
        self.path.clone()
    }

    fn content_type(&self) -> Option<String> {
        self.content_type.clone()
    }
}
