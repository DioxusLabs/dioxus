use crate::{file_transfer::RemoteFile, upload::StoredFile};
use bytes::Bytes;
use dioxus_core::CapturedError;
use dioxus_html::{
    FileData, FormValue, HasFileData, HasFormData, NativeFileData, SerializedFileData,
    SerializedFormData,
};
use futures_util::Stream;
use std::{any::Any, future::Future, io::Read, path::PathBuf, pin::Pin, sync::Arc};

#[derive(Clone)]
pub(crate) struct LiveviewFormData {
    value: String,
    valid: bool,
    values: Vec<(String, FormValue)>,
}

impl LiveviewFormData {
    pub(crate) fn new(form: SerializedFormData, files: Vec<FileStorage>) -> Self {
        let mut files = files.into_iter();
        let values = form
            .values
            .into_iter()
            .map(|value| {
                let file = value.file.map(|metadata| LiveviewFileData {
                    metadata,
                    storage: files.next().unwrap_or(FileStorage::Unavailable),
                });
                let data = if let Some(text) = value.text {
                    FormValue::Text(text)
                } else if let Some(file) = file {
                    FormValue::File(Some(FileData::new(file)))
                } else {
                    FormValue::File(None)
                };
                (value.key, data)
            })
            .collect();
        Self {
            value: form.value,
            valid: form.valid,
            values,
        }
    }
}

impl HasFormData for LiveviewFormData {
    fn value(&self) -> String {
        self.value.clone()
    }

    fn valid(&self) -> bool {
        self.valid
    }

    fn values(&self) -> Vec<(String, FormValue)> {
        self.values.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl HasFileData for LiveviewFormData {
    fn files(&self) -> Vec<FileData> {
        self.values
            .iter()
            .filter_map(|(_, value)| match value {
                FormValue::File(file) => file.clone(),
                FormValue::Text(_) => None,
            })
            .collect()
    }
}

struct LiveviewFileData {
    metadata: SerializedFileData,
    storage: FileStorage,
}

#[derive(Clone)]
pub(crate) enum FileStorage {
    Available(Arc<StoredFile>),
    Remote(Arc<RemoteFile>),
    Unavailable,
}

impl FileStorage {
    async fn read(self) -> Result<Arc<StoredFile>, CapturedError> {
        match self {
            Self::Available(file) => Ok(file),
            Self::Remote(file) => file.read().await.map_err(CapturedError::msg),
            Self::Unavailable => Err(CapturedError::msg("File contents not available")),
        }
    }

    fn stored(&self) -> Option<&Arc<StoredFile>> {
        match self {
            Self::Available(file) => Some(file),
            Self::Remote(file) => file.stored(),
            Self::Unavailable => None,
        }
    }
}

impl NativeFileData for LiveviewFileData {
    fn name(&self) -> String {
        self.metadata.name.clone()
    }

    fn size(&self) -> u64 {
        self.metadata.size
    }

    fn last_modified(&self) -> u64 {
        self.metadata.last_modified
    }

    fn path(&self) -> PathBuf {
        self.storage
            .stored()
            .map_or_else(PathBuf::new, |file| file.path.to_path_buf())
    }

    fn content_type(&self) -> Option<String> {
        self.metadata.content_type.clone()
    }

    fn read_bytes(&self) -> Pin<Box<dyn Future<Output = Result<Bytes, CapturedError>>>> {
        let storage = self.storage.clone();
        Box::pin(async move {
            let storage = storage.read().await?;
            Ok(
                tokio::task::spawn_blocking(move || std::fs::read(&storage.path))
                    .await??
                    .into(),
            )
        })
    }

    fn read_string(&self) -> Pin<Box<dyn Future<Output = Result<String, CapturedError>>>> {
        let storage = self.storage.clone();
        Box::pin(async move {
            let storage = storage.read().await?;
            Ok(
                tokio::task::spawn_blocking(move || std::fs::read_to_string(&storage.path))
                    .await??,
            )
        })
    }

    fn byte_stream(&self) -> Pin<Box<dyn Stream<Item = Result<Bytes, CapturedError>> + Send>> {
        let state = (None::<std::fs::File>, self.storage.clone());
        Box::pin(futures_util::stream::try_unfold(
            state,
            |(file, storage)| async move {
                let storage = storage.read().await?;
                tokio::task::spawn_blocking(move || {
                    let mut file = match file {
                        Some(file) => file,
                        None => std::fs::File::open(&storage.path)?,
                    };
                    let mut bytes = vec![0; 64 * 1024];
                    let count = file.read(&mut bytes)?;
                    if count == 0 {
                        return Ok(None);
                    }
                    bytes.truncate(count);
                    Ok::<_, CapturedError>(Some((
                        Bytes::from(bytes),
                        (Some(file), FileStorage::Available(storage)),
                    )))
                })
                .await?
            },
        ))
    }

    fn inner(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upload::{FileUploadRegistry, UploadError, UploadSession};
    use futures_util::StreamExt;

    #[test]
    fn unselected_files_and_empty_text_match_serialized_form_data() {
        #[derive(serde::Deserialize)]
        struct Fields {
            description: String,
            upload: Option<SerializedFileData>,
        }

        let serialized: SerializedFormData = serde_json::from_value(serde_json::json!({
            "values": [
                { "key": "description", "text": "" },
                { "key": "upload" }
            ]
        }))
        .unwrap();
        for form in [
            dioxus_html::FormData::new(serialized.clone()),
            dioxus_html::FormData::new(LiveviewFormData::new(serialized, Vec::new())),
        ] {
            assert_eq!(
                form.get_first("description"),
                Some(FormValue::Text(String::new()))
            );
            assert_eq!(form.get_first("upload"), Some(FormValue::File(None)));
            assert!(form.get_first("missing").is_none());
            assert!(form.files().is_empty());
            let fields: Fields = form.deserialize_values().unwrap();
            assert!(fields.description.is_empty());
            assert_eq!(fields.upload, None);

            let json = serde_json::to_value(form).unwrap();
            assert_eq!(json["values"][1]["file"], serde_json::Value::Null);
            let restored: dioxus_html::FormData = serde_json::from_value(json).unwrap();
            assert_eq!(restored.get_first("upload"), Some(FormValue::File(None)));
            assert!(restored.files().is_empty());
        }
    }

    #[test]
    fn buffered_metadata_preserves_names_and_duplicates() {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum BufferedFile {
            File(SerializedFileData),
        }

        #[derive(serde::Deserialize)]
        struct Fields {
            files: Vec<BufferedFile>,
        }

        // Preserve names without resolving metadata back to handles, even for duplicates.
        let names = ["first.txt", "second.txt", "first.txt", ""];
        let serialized = SerializedFormData::new(
            String::new(),
            names
                .into_iter()
                .map(|name| dioxus_html::SerializedFormObject {
                    key: "files".into(),
                    text: None,
                    file: Some(SerializedFileData {
                        name: name.into(),
                        size: 5,
                        ..SerializedFileData::empty()
                    }),
                })
                .collect(),
        );
        let form = dioxus_html::FormData::new(LiveviewFormData::new(serialized, Vec::new()));
        let fields: Fields = form.deserialize_values().unwrap();
        assert_eq!(fields.files.len(), names.len());
        for (BufferedFile::File(file), name) in fields.files.into_iter().zip(names) {
            assert!(file.path.as_os_str().is_empty());
            assert_eq!(file.name, name);
            assert_eq!(file.size, 5);
        }

        let Some(FormValue::File(Some(first))) = form.get_first("files") else {
            panic!("expected a selected file");
        };
        let files = form.files();
        drop(form);
        assert_eq!(first.name(), "first.txt");
        assert_eq!(files[1].name(), "second.txt");
    }

    async fn uploaded_file(contents: Bytes) -> (FileData, FileUploadRegistry, UploadSession) {
        let registry = FileUploadRegistry::new(contents.len() as u64);
        let session = registry.new_session();
        let reservation = registry.reserve(&session, contents.len() as u64).unwrap();
        let token = registry.register_reserved(reservation);
        let mut writer = registry.begin(&token, None).await.unwrap();
        writer.write(contents.clone()).await.unwrap();
        writer.finish().unwrap();
        let storage = registry.take_completed(&token).unwrap();
        let file = FileData::new(LiveviewFileData {
            metadata: SerializedFileData {
                name: "browser-name.txt".into(),
                size: contents.len() as u64,
                ..SerializedFileData::empty()
            },
            storage: FileStorage::Available(storage),
        });
        (file, registry, session)
    }

    #[tokio::test]
    async fn parsed_uploaded_metadata_preserves_name_without_owning_storage() {
        #[derive(serde::Deserialize)]
        struct Fields {
            upload: SerializedFileData,
        }

        let (file, _registry, _session) = uploaded_file(Bytes::from_static(b"hello")).await;
        let path = file.path();
        let form = dioxus_html::FormData::new(LiveviewFormData {
            value: String::new(),
            valid: true,
            values: vec![("upload".into(), FormValue::File(Some(file)))],
        });
        let fields: Fields = form.deserialize_values().unwrap();
        assert_eq!(fields.upload.name, "browser-name.txt");
        assert_eq!(fields.upload.path, path);
        drop(form);
        assert!(!path.exists());
        assert_eq!(fields.upload.name, "browser-name.txt");
    }

    #[tokio::test]
    async fn byte_stream_keeps_file_and_quota_alive_and_reads_in_chunks() {
        let contents = Bytes::from(vec![42; 150_000]);
        let (file, registry, session) = uploaded_file(contents.clone()).await;
        let path = file.path();
        let mut stream = file.byte_stream();
        drop(file);
        assert!(path.exists());
        assert_eq!(
            registry.reserve(&session, 1).err(),
            Some(UploadError::LimitExceeded)
        );
        let mut total = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            assert!(!chunk.is_empty() && chunk.len() <= 64 * 1024);
            assert_eq!(chunk, contents.slice(total..total + chunk.len()));
            total += chunk.len();
            assert!(path.exists());
            assert_eq!(
                registry.reserve(&session, 1).err(),
                Some(UploadError::LimitExceeded)
            );
        }
        assert_eq!(total, contents.len());
        drop(stream);
        assert!(!path.exists());
        assert!(registry.reserve(&session, contents.len() as u64).is_ok());
    }

    #[tokio::test]
    async fn dropping_a_partially_read_stream_deletes_the_file() {
        let (file, registry, session) = uploaded_file(Bytes::from(vec![42; 150_000])).await;
        let path = file.path();
        let mut stream = file.byte_stream();
        drop(file);
        assert_eq!(stream.next().await.unwrap().unwrap().len(), 64 * 1024);
        assert!(path.exists());
        drop(stream);
        assert!(!path.exists());
        assert!(registry.reserve(&session, 150_000).is_ok());
    }

    #[tokio::test]
    async fn read_futures_keep_the_temporary_file_alive() {
        let (file, registry, session) = uploaded_file(Bytes::from_static(b"hello")).await;
        let path = file.path();
        let native = file.inner().downcast_ref::<LiveviewFileData>().unwrap();
        let bytes = native.read_bytes();
        let text = native.read_string();
        drop(file);
        assert!(path.exists());
        assert_eq!(bytes.await.unwrap(), b"hello"[..]);
        assert_eq!(
            registry.reserve(&session, 1).err(),
            Some(UploadError::LimitExceeded)
        );
        assert_eq!(text.await.unwrap(), "hello");
        assert!(!path.exists());
        assert!(registry.reserve(&session, 5).is_ok());
    }

    #[tokio::test]
    async fn metadata_only_events_cannot_read_client_supplied_server_paths() {
        let server_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(server_file.path(), b"server data").unwrap();
        let form = LiveviewFormData::new(
            SerializedFormData::new(
                String::new(),
                vec![dioxus_html::SerializedFormObject {
                    key: "file".to_string(),
                    text: None,
                    file: Some(SerializedFileData {
                        name: "client-file.txt".into(),
                        path: server_file.path().to_path_buf(),
                        size: 11,
                        ..SerializedFileData::empty()
                    }),
                }],
            ),
            Vec::new(),
        );
        let file = form.files().pop().unwrap();
        assert!(file.path().as_os_str().is_empty());
        assert!(file.read_bytes().await.is_err());
        assert!(file.read_string().await.is_err());
        assert!(file.byte_stream().next().await.unwrap().is_err());
    }
}
