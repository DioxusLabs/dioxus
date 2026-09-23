use bytes::Bytes;
use futures_util::Stream;
use std::{path::PathBuf, pin::Pin, prelude::rust_2024::Future};

/// An owned file handle provided by a renderer.
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

    /// Returns the filename reported by the renderer.
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

    /// Returns a filesystem path, or an empty path when unavailable.
    ///
    /// - Desktop returns the local filesystem path.
    /// - Web always returns an empty path. Use [`Self::name`] for the filename.
    /// - Liveview returns an empty path until the upload completes successfully, then
    ///   the server's temporary file path. The path stays empty while the upload is
    ///   pending or if the transfer fails or the contents are unavailable. An upload is
    ///   lazily triggered by the first read of the data. Keep an original liveview file handle
    ///   alive while using its temporary path. Use [`Self::name`] to obtain the browser's filename.
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

    /// A serializable representation of file metadata
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
    pub struct SerializedFileData {
        /// The original filename, independent of the file's storage path.
        pub name: String,
        pub path: PathBuf,
        pub size: u64,
        pub last_modified: u64,
        pub content_type: Option<String>,
    }

    impl SerializedFileData {
        /// Create a new empty serialized file data object
        pub fn empty() -> Self {
            Self {
                name: String::new(),
                path: PathBuf::new(),
                size: 0,
                last_modified: 0,
                content_type: None,
            }
        }

        /// Create serialized file metadata without eagerly reading the file contents.
        pub(crate) fn from_file_data(file_data: &FileData) -> Self {
            Self {
                name: file_data.name(),
                path: file_data.path(),
                size: file_data.size(),
                last_modified: file_data.last_modified(),
                content_type: file_data.content_type(),
            }
        }
    }

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
        ) -> Pin<Box<dyn Future<Output = Result<Bytes, dioxus_core::CapturedError>> + 'static>>
        {
            let path = self.path.clone();

            Box::pin(async move {
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
            let path = self.path.clone();

            Box::pin(async move {
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
            let path = self.path.clone();

            Box::pin(futures_util::stream::once(async move {
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

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn serialized_names_remain_independent_of_paths() {
            for (fields, expected) in [
                (
                    serde_json::json!({"name": "report.pdf", "path": "/tmp/upload-123"}),
                    "report.pdf",
                ),
                (
                    serde_json::json!({"name": "report.pdf", "path": ""}),
                    "report.pdf",
                ),
                (
                    serde_json::json!({"name": "", "path": "/tmp/upload-123"}),
                    "",
                ),
                (serde_json::json!({"name": "", "path": ""}), ""),
            ] {
                let mut value = serde_json::to_value(SerializedFileData::empty()).unwrap();
                value
                    .as_object_mut()
                    .unwrap()
                    .extend(fields.as_object().unwrap().clone());
                let metadata: SerializedFileData = serde_json::from_value(value).unwrap();
                assert_eq!(metadata.name(), expected);

                let snapshot = SerializedFileData::from_file_data(&FileData::new(metadata));
                let json = serde_json::to_value(snapshot).unwrap();
                assert_eq!(json["name"], expected);
                let round_trip: SerializedFileData = serde_json::from_value(json).unwrap();
                assert_eq!(round_trip.name, expected);
            }
        }
    }
}
