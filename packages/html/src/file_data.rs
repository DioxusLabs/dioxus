use bytes::Bytes;
use futures_util::Stream;
use std::{path::PathBuf, pin::Pin, prelude::rust_2024::Future};

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

    pub async fn read_bytes(&self) -> Result<Bytes, dioxus_core::CapturedError> {
        self.inner.read_bytes().await
    }

    pub async fn read_string(&self) -> Result<String, dioxus_core::CapturedError> {
        self.inner.read_string().await
    }

    pub fn byte_stream(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, dioxus_core::CapturedError>> + Send + 'static>>
    {
        self.inner.byte_stream()
    }

    pub fn inner(&self) -> &dyn std::any::Any {
        self.inner.inner()
    }

    pub fn path(&self) -> PathBuf {
        self.inner.path()
    }
}

impl PartialEq for FileData {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
            && self.size() == other.size()
            && self.last_modified() == other.last_modified()
            && self.path() == other.path()
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
    ) -> Pin<Box<dyn Future<Output = Result<Bytes, dioxus_core::CapturedError>> + 'static>>;
    fn byte_stream(
        &self,
    ) -> Pin<
        Box<
            dyn futures_util::Stream<Item = Result<Bytes, dioxus_core::CapturedError>>
                + 'static
                + Send,
        >,
    >;
    fn read_string(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<String, dioxus_core::CapturedError>> + 'static>>;
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

pub trait HasFileData: std::any::Any {
    fn files(&self) -> Vec<FileData>;
}

#[cfg(feature = "serialize")]
pub use serialize::*;

#[cfg(feature = "serialize")]
mod serialize {
    use super::*;

    /// A serializable representation of file data
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
    pub struct SerializedFileData {
        pub path: PathBuf,
        pub size: u64,
        pub last_modified: u64,
        pub content_type: Option<String>,
        pub contents: Option<bytes::Bytes>,
    }

    impl SerializedFileData {
        /// Create a new empty serialized file data object
        pub fn empty() -> Self {
            Self {
                path: PathBuf::new(),
                size: 0,
                last_modified: 0,
                content_type: None,
                contents: None,
            }
        }
    }

    impl NativeFileData for SerializedFileData {
        fn name(&self) -> String {
            self.path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        }

        fn size(&self) -> u64 {
            self.size
        }

        fn last_modified(&self) -> u64 {
            self.last_modified
        }

        fn read_bytes(
            &self,
        ) -> Pin<Box<dyn Future<Output = Result<Bytes, dioxus_core::CapturedError>> + 'static>>
        {
            let contents = self.contents.clone();
            let path = self.path.clone();

            Box::pin(async move {
                if let Some(contents) = contents {
                    return Ok(contents);
                }

                #[cfg(not(target_arch = "wasm32"))]
                if path.exists() {
                    return Ok(std::fs::read(path).map(Bytes::from)?);
                }

                Err(dioxus_core::CapturedError::msg(
                    "File contents not available",
                ))
            })
        }

        fn read_string(
            &self,
        ) -> Pin<Box<dyn Future<Output = Result<String, dioxus_core::CapturedError>> + 'static>>
        {
            let contents = self.contents.clone();
            let path = self.path.clone();

            Box::pin(async move {
                if let Some(contents) = contents {
                    return Ok(String::from_utf8(contents.to_vec())?);
                }

                #[cfg(not(target_arch = "wasm32"))]
                if path.exists() {
                    return Ok(std::fs::read_to_string(path)?);
                }

                Err(dioxus_core::CapturedError::msg(
                    "File contents not available",
                ))
            })
        }

        fn byte_stream(
            &self,
        ) -> Pin<
            Box<
                dyn futures_util::Stream<Item = Result<Bytes, dioxus_core::CapturedError>>
                    + 'static
                    + Send,
            >,
        > {
            let contents = self.contents.clone();
            let path = self.path.clone();

            Box::pin(futures_util::stream::once(async move {
                if let Some(contents) = contents {
                    return Ok(contents);
                }

                #[cfg(not(target_arch = "wasm32"))]
                if path.exists() {
                    return Ok(std::fs::read(path).map(Bytes::from)?);
                }

                Err(dioxus_core::CapturedError::msg(
                    "File contents not available",
                ))
            }))
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

    impl<'de> serde::Deserialize<'de> for FileData {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let sfd = SerializedFileData::deserialize(deserializer)?;
            Ok(FileData::new(sfd))
        }
    }
}
