#![cfg_attr(not(feature = "axum"), allow(dead_code))]

use bytes::Bytes;
use std::{
    collections::HashMap,
    io::Write,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tempfile::{NamedTempFile, TempPath};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, thiserror::Error, PartialEq)]
pub(crate) enum UploadError {
    #[error("unknown or expired LiveView file upload")]
    UnknownOrExpired,
    #[error("LiveView file upload has already started")]
    AlreadyStarted,
    #[error("LiveView file upload was canceled")]
    Canceled,
    #[error("failed to store LiveView file upload: {0}")]
    StorageFailed(String),
    #[error("LiveView file upload exceeds the connection's upload data limit")]
    LimitExceeded,
    #[error("LiveView file upload exceeds the connection's file count limit")]
    FileCountLimitExceeded,
    #[error("failed to read LiveView file upload body")]
    BodyReadFailed,
    #[error("LiveView file upload size did not match the declared size")]
    SizeMismatch,
    #[error(
        "LiveView did not receive the file through its HTTP upload handler; mount axum_file_upload at the WebSocket path followed by /upload/{{token}} using the same LiveViewPool"
    )]
    Incomplete,
}

fn storage_failed(error: impl ToString) -> UploadError {
    UploadError::StorageFailed(error.to_string())
}

#[derive(Clone)]
pub(crate) struct FileUploadRegistry {
    uploads: Arc<Mutex<HashMap<String, RegisteredUpload>>>,
    data_limit: u64,
    file_limit: usize,
    timeout: Duration,
}

pub(crate) struct UploadSession {
    reserved_bytes: Arc<AtomicU64>,
    reserved_files: Arc<AtomicUsize>,
    data_limit: u64,
    file_limit: usize,
}

struct RegisteredUpload {
    state: Arc<Mutex<UploadState>>,
    expires: Instant,
    canceled: CancellationToken,
    reservation: Arc<UploadReservation>,
}

impl RegisteredUpload {
    fn is_expired(&self, now: Instant) -> bool {
        self.expires <= now && matches!(*self.state.lock().unwrap(), UploadState::Pending)
    }
}

pub(crate) struct UploadReservation {
    bytes: u64,
    reserved_bytes: Arc<AtomicU64>,
    reserved_files: Arc<AtomicUsize>,
}

struct TemporaryUpload {
    file: NamedTempFile,
    reservation: Arc<UploadReservation>,
}

pub(crate) struct StoredFile {
    pub(crate) path: TempPath,
    _reservation: Arc<UploadReservation>,
}

enum UploadState {
    Pending,
    Uploading,
    Complete(Arc<StoredFile>),
    Failed,
}

pub(crate) struct UploadWriter {
    expected_size: u64,
    written: u64,
    file: Option<Arc<TemporaryUpload>>,
    canceled: CancellationToken,
    state: Arc<Mutex<UploadState>>,
    finished: bool,
}

impl Default for FileUploadRegistry {
    fn default() -> Self {
        Self::new(crate::DEFAULT_UPLOAD_STORAGE_LIMIT)
    }
}

impl FileUploadRegistry {
    pub(crate) fn new(data_limit: u64) -> Self {
        Self {
            uploads: Default::default(),
            data_limit,
            file_limit: crate::DEFAULT_UPLOAD_FILE_LIMIT,
            timeout: crate::DEFAULT_UPLOAD_TIMEOUT,
        }
    }

    pub(crate) fn with_limit(self, data_limit: u64) -> Self {
        Self {
            uploads: Default::default(),
            data_limit,
            ..self
        }
    }

    pub(crate) fn with_timeout(self, timeout: Duration) -> Self {
        Self {
            uploads: Default::default(),
            timeout,
            ..self
        }
    }

    pub(crate) fn with_file_limit(self, file_limit: usize) -> Self {
        Self {
            uploads: Default::default(),
            file_limit,
            ..self
        }
    }

    pub(crate) fn new_session(&self) -> UploadSession {
        UploadSession {
            reserved_bytes: Default::default(),
            reserved_files: Default::default(),
            data_limit: self.data_limit,
            file_limit: self.file_limit,
        }
    }

    pub(crate) fn reserve(
        &self,
        session: &UploadSession,
        size: u64,
    ) -> Result<Arc<UploadReservation>, UploadError> {
        let now = Instant::now();
        self.uploads.lock().unwrap().retain(|_, upload| {
            if upload.is_expired(now) {
                upload.canceled.cancel();
                false
            } else {
                true
            }
        });
        session.reserve(size).map(Arc::new)
    }

    pub(crate) fn register_reserved(&self, reservation: Arc<UploadReservation>) -> String {
        let token = Uuid::new_v4().to_string();
        self.uploads.lock().unwrap().insert(
            token.clone(),
            RegisteredUpload {
                state: Arc::new(Mutex::new(UploadState::Pending)),
                expires: Instant::now() + self.timeout,
                canceled: CancellationToken::new(),
                reservation,
            },
        );
        token
    }

    pub(crate) async fn wait_for_cleanup(&self, token: &str) {
        let (expires, canceled) = {
            let registry = self.uploads.lock().unwrap();
            let Some(upload) = registry.get(token) else {
                return;
            };
            (upload.expires, upload.canceled.clone())
        };
        tokio::select! {
            _ = canceled.cancelled() => {}
            _ = tokio::time::sleep_until(expires) => {
                let mut registry = self.uploads.lock().unwrap();
                let Some(upload) = registry.get(token) else {
                    return;
                };
                if upload.is_expired(Instant::now()) {
                    let upload = registry.remove(token).unwrap();
                    upload.canceled.cancel();
                    return;
                }
            }
        }
        // An HTTP request that started before the deadline remains valid until canceled.
        canceled.cancelled().await;
        self.cancel(token);
    }

    pub(crate) async fn begin(
        &self,
        token: &str,
        content_length: Option<u64>,
    ) -> Result<UploadWriter, UploadError> {
        let (canceled, state, reservation) = {
            let registry = self.uploads.lock().unwrap();
            let upload = registry.get(token).ok_or(UploadError::UnknownOrExpired)?;
            if upload.is_expired(Instant::now()) {
                upload.canceled.cancel();
                return Err(UploadError::UnknownOrExpired);
            }
            if upload.canceled.is_cancelled() {
                return Err(UploadError::Canceled);
            }
            let mut state = upload.state.lock().unwrap();
            if !matches!(*state, UploadState::Pending) {
                return Err(UploadError::AlreadyStarted);
            }
            if content_length.is_some_and(|size| size != upload.reservation.bytes) {
                *state = UploadState::Failed;
                upload.canceled.cancel();
                return Err(UploadError::SizeMismatch);
            }
            *state = UploadState::Uploading;
            (
                upload.canceled.clone(),
                upload.state.clone(),
                upload.reservation.clone(),
            )
        };
        let mut writer = UploadWriter {
            expected_size: reservation.bytes,
            written: 0,
            file: None,
            canceled,
            state,
            finished: false,
        };
        writer.file = Some(
            tokio::task::spawn_blocking(move || {
                let file = tempfile::Builder::new()
                    .prefix("dioxus-liveview-")
                    .tempfile()
                    .map_err(storage_failed)?;
                Ok::<_, UploadError>(Arc::new(TemporaryUpload { file, reservation }))
            })
            .await
            .map_err(storage_failed)??,
        );
        if writer.canceled.is_cancelled() {
            return Err(UploadError::Canceled);
        }
        Ok(writer)
    }

    pub(crate) fn take_completed(&self, token: &str) -> Result<Arc<StoredFile>, UploadError> {
        let mut registry = self.uploads.lock().unwrap();
        let file = {
            let upload = registry.get(token).ok_or(UploadError::UnknownOrExpired)?;
            if upload.canceled.is_cancelled() {
                return Err(UploadError::Canceled);
            }
            let mut state = upload.state.lock().unwrap();
            if !matches!(*state, UploadState::Complete(_)) {
                return Err(UploadError::Incomplete);
            }
            let UploadState::Complete(file) = std::mem::replace(&mut *state, UploadState::Failed)
            else {
                unreachable!();
            };
            file
        };
        registry.remove(token);
        Ok(file)
    }

    pub(crate) fn cancel(&self, token: &str) {
        if let Some(upload) = self.uploads.lock().unwrap().remove(token) {
            upload.canceled.cancel();
            *upload.state.lock().unwrap() = UploadState::Failed;
        }
    }
}

impl UploadSession {
    fn reserve(&self, size: u64) -> Result<UploadReservation, UploadError> {
        self.reserved_files
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |reserved| {
                reserved
                    .checked_add(1)
                    .filter(|count| *count <= self.file_limit)
            })
            .map_err(|_| UploadError::FileCountLimitExceeded)?;
        // Construct the reservation first so a failed byte reservation releases the file slot.
        let mut reservation = UploadReservation {
            bytes: 0,
            reserved_bytes: self.reserved_bytes.clone(),
            reserved_files: self.reserved_files.clone(),
        };
        self.reserved_bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |reserved| {
                reserved
                    .checked_add(size)
                    .filter(|bytes| *bytes <= self.data_limit)
            })
            .map_err(|_| UploadError::LimitExceeded)?;
        reservation.bytes = size;
        Ok(reservation)
    }
}

impl Drop for UploadReservation {
    fn drop(&mut self) {
        self.reserved_bytes.fetch_sub(self.bytes, Ordering::AcqRel);
        self.reserved_files.fetch_sub(1, Ordering::AcqRel);
    }
}

impl UploadWriter {
    pub(crate) async fn cancelled(&self) {
        self.canceled.cancelled().await;
    }

    pub(crate) async fn write(&mut self, bytes: Bytes) -> Result<(), UploadError> {
        if self.canceled.is_cancelled() {
            return Err(UploadError::Canceled);
        }
        let next = self
            .written
            .checked_add(bytes.len() as u64)
            .filter(|size| *size <= self.expected_size)
            .ok_or(UploadError::SizeMismatch)?;
        let file = self
            .file
            .as_ref()
            .ok_or(UploadError::AlreadyStarted)?
            .clone();
        // The disk operation keeps the file and its quota alive even if this future is dropped.
        tokio::task::spawn_blocking(move || file.file.as_file().write_all(&bytes))
            .await
            .map_err(storage_failed)?
            .map_err(storage_failed)?;
        if self.canceled.is_cancelled() {
            return Err(UploadError::Canceled);
        }
        self.written = next;
        Ok(())
    }

    pub(crate) fn finish(mut self) -> Result<(), UploadError> {
        if self.canceled.is_cancelled() {
            return Err(UploadError::Canceled);
        }
        if self.written != self.expected_size {
            return Err(UploadError::SizeMismatch);
        }
        let mut state = self.state.lock().unwrap();
        if !matches!(*state, UploadState::Uploading) {
            return Err(UploadError::Canceled);
        }
        let file = Arc::try_unwrap(self.file.take().ok_or(UploadError::AlreadyStarted)?)
            .map_err(|_| storage_failed("upload is still writing"))?;
        *state = UploadState::Complete(Arc::new(StoredFile {
            path: file.file.into_temp_path(),
            _reservation: file.reservation,
        }));
        self.finished = true;
        Ok(())
    }
}

impl Drop for UploadWriter {
    fn drop(&mut self) {
        if !self.finished {
            self.canceled.cancel();
            *self.state.lock().unwrap() = UploadState::Failed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DEFAULT_UPLOAD_FILE_LIMIT, DEFAULT_UPLOAD_STORAGE_LIMIT, DEFAULT_UPLOAD_TIMEOUT,
        LiveViewPool,
    };

    #[test]
    fn registration_does_not_allocate_the_file_size() {
        let registry = FileUploadRegistry::new(u64::MAX);
        let session = registry.new_session();
        let reservation = registry.reserve(&session, u64::MAX).unwrap();
        let token = registry.register_reserved(reservation);
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), u64::MAX);
        assert_eq!(
            registry.reserve(&session, 1).err(),
            Some(UploadError::LimitExceeded)
        );
        assert_eq!(session.reserved_files.load(Ordering::Acquire), 1);
        registry.cancel(&token);
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 0);
        assert_eq!(session.reserved_files.load(Ordering::Acquire), 0);
    }

    #[test]
    fn registration_limits_zero_byte_file_count() {
        let registry = FileUploadRegistry::default().with_file_limit(2);
        let session = registry.new_session();
        let first = registry.reserve(&session, 0).unwrap();
        let second = registry.reserve(&session, 0).unwrap();
        assert_eq!(
            registry.reserve(&session, 0).err(),
            Some(UploadError::FileCountLimitExceeded)
        );
        assert!(registry.uploads.lock().unwrap().is_empty());
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 0);
        assert!(registry.reserve(&registry.new_session(), 0).is_ok());
        drop(first);
        assert!(registry.reserve(&session, 0).is_ok());
        drop(second);
        assert_eq!(session.reserved_files.load(Ordering::Acquire), 0);
    }

    #[tokio::test]
    async fn retained_files_count_toward_their_connections_limit() {
        let registry = FileUploadRegistry::new(3);
        let session = registry.new_session();
        let other_session = registry.new_session();
        let mut files = Vec::new();
        for contents in [&b"ab"[..], &b"c"[..]] {
            let reservation = registry.reserve(&session, contents.len() as u64).unwrap();
            let token = registry.register_reserved(reservation);
            let mut upload = registry.begin(&token, None).await.unwrap();
            upload.write(Bytes::from_static(contents)).await.unwrap();
            upload.finish().unwrap();
            files.push(registry.take_completed(&token).unwrap());
        }
        let paths: Vec<_> = files.iter().map(|file| file.path.to_path_buf()).collect();
        let first = files.remove(0);
        let retained = first.clone();
        drop(first);
        assert!(paths.iter().all(|path| path.exists()));
        assert_eq!(
            registry.reserve(&session, 1).err(),
            Some(UploadError::LimitExceeded)
        );
        let reservation = registry.reserve(&other_session, 3).unwrap();
        let other_token = registry.register_reserved(reservation);
        drop(retained);
        assert!(!paths[0].exists());
        assert!(paths[1].exists());
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 1);
        let reservation = registry.reserve(&session, 2).unwrap();
        let incoming = registry.register_reserved(reservation);
        assert_eq!(
            registry.reserve(&session, 1).err(),
            Some(UploadError::LimitExceeded)
        );
        drop(files);
        assert!(!paths[1].exists());
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 2);
        registry.cancel(&incoming);
        registry.cancel(&other_token);
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 0);
    }

    #[tokio::test]
    async fn incomplete_uploads_delete_their_temporary_files() {
        let registry = FileUploadRegistry::new(3);
        let session = registry.new_session();
        let reservation = registry.reserve(&session, 3).unwrap();
        let token = registry.register_reserved(reservation);
        let mut upload = registry.begin(&token, None).await.unwrap();
        let path = upload.file.as_ref().unwrap().file.path().to_path_buf();
        upload.write(Bytes::from_static(b"ab")).await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"ab");
        assert_eq!(upload.finish(), Err(UploadError::SizeMismatch));
        assert!(!path.exists());
        registry.wait_for_cleanup(&token).await;
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 0);
    }

    #[test]
    fn canceled_disk_writes_keep_their_file_and_quota_until_they_stop() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .max_blocking_threads(1)
            .build()
            .unwrap()
            .block_on(async {
                let registry = FileUploadRegistry::new(3);
                let session = registry.new_session();
                let reservation = registry.reserve(&session, 3).unwrap();
                let token = registry.register_reserved(reservation);
                let mut upload = registry.begin(&token, None).await.unwrap();
                let path = upload.file.as_ref().unwrap().file.path().to_path_buf();
                let (started_tx, started_rx) = tokio::sync::oneshot::channel();
                let (release_tx, release_rx) = std::sync::mpsc::channel();
                let blocker = tokio::task::spawn_blocking(move || {
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                });
                started_rx.await.unwrap();
                {
                    let write = upload.write(Bytes::from_static(b"abc"));
                    futures_util::pin_mut!(write);
                    assert!(futures_util::poll!(&mut write).is_pending());
                }
                registry.cancel(&token);
                drop(upload);
                assert!(path.exists());
                assert_eq!(
                    registry.reserve(&session, 1).err(),
                    Some(UploadError::LimitExceeded)
                );
                release_tx.send(()).unwrap();
                blocker.await.unwrap();
                tokio::task::spawn_blocking(|| {}).await.unwrap();
                assert!(!path.exists());
                assert!(registry.reserve(&session, 3).is_ok());
            });
    }

    #[tokio::test(start_paused = true)]
    async fn omitted_upload_settings_keep_their_defaults() {
        let pool = LiveViewPool::new();
        let timeout_only = pool.clone().with_upload_timeout(Duration::from_secs(1));
        assert_eq!(
            timeout_only
                .uploads
                .reserve(
                    &timeout_only.uploads.new_session(),
                    DEFAULT_UPLOAD_STORAGE_LIMIT + 1
                )
                .err(),
            Some(UploadError::LimitExceeded)
        );
        let session = timeout_only.uploads.new_session();
        let reservations: Vec<_> = (0..DEFAULT_UPLOAD_FILE_LIMIT)
            .map(|_| timeout_only.uploads.reserve(&session, 0).unwrap())
            .collect();
        assert_eq!(
            timeout_only.uploads.reserve(&session, 0).err(),
            Some(UploadError::FileCountLimitExceeded)
        );
        drop(reservations);

        let file_limit_only = pool.clone().with_upload_file_limit(2);
        let session = file_limit_only.uploads.new_session();
        let first = file_limit_only.uploads.reserve(&session, 0).unwrap();
        let second = file_limit_only.uploads.reserve(&session, 0).unwrap();
        assert_eq!(
            file_limit_only.uploads.reserve(&session, 0).err(),
            Some(UploadError::FileCountLimitExceeded)
        );
        drop((first, second));

        let limit_only = pool.with_upload_storage_limit(2);
        let session = limit_only.uploads.new_session();
        let reservation = limit_only.uploads.reserve(&session, 2).unwrap();
        let token = limit_only.uploads.register_reserved(reservation);
        let expiration = limit_only.uploads.wait_for_cleanup(&token);
        futures_util::pin_mut!(expiration);
        assert!(futures_util::poll!(&mut expiration).is_pending());
        tokio::time::advance(DEFAULT_UPLOAD_TIMEOUT - Duration::from_secs(1)).await;
        assert!(futures_util::poll!(&mut expiration).is_pending());
        tokio::time::advance(Duration::from_secs(2)).await;
        assert!(futures_util::poll!(&mut expiration).is_ready());
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn pools_can_use_independent_upload_limits_and_timeouts() {
        let first = LiveViewPool::new()
            .with_upload_storage_limit(2)
            .with_upload_timeout(Duration::from_secs(10));
        let second = first
            .clone()
            .with_upload_timeout(Duration::from_secs(20))
            .with_upload_storage_limit(3);
        let first_session = first.uploads.new_session();
        let second_session = second.uploads.new_session();
        let reservation = first.uploads.reserve(&first_session, 2).unwrap();
        let first_token = first.uploads.register_reserved(reservation);
        let reservation = second.uploads.reserve(&second_session, 3).unwrap();
        let second_token = second.uploads.register_reserved(reservation);
        for (pool, session) in [(&first, &first_session), (&second, &second_session)] {
            assert_eq!(
                pool.uploads.reserve(session, 1).err(),
                Some(UploadError::LimitExceeded)
            );
        }

        let first_expiration = first.uploads.wait_for_cleanup(&first_token);
        let second_expiration = second.uploads.wait_for_cleanup(&second_token);
        futures_util::pin_mut!(first_expiration, second_expiration);
        assert!(futures_util::poll!(&mut first_expiration).is_pending());
        assert!(futures_util::poll!(&mut second_expiration).is_pending());

        tokio::time::advance(Duration::from_secs(11)).await;
        assert!(futures_util::poll!(&mut first_expiration).is_ready());
        assert!(futures_util::poll!(&mut second_expiration).is_pending());
        assert_eq!(first_session.reserved_bytes.load(Ordering::Acquire), 0);
        assert_eq!(second_session.reserved_bytes.load(Ordering::Acquire), 3);

        tokio::time::advance(Duration::from_secs(10)).await;
        assert!(futures_util::poll!(&mut second_expiration).is_ready());
        assert_eq!(second_session.reserved_bytes.load(Ordering::Acquire), 0);
    }

    #[tokio::test]
    async fn uploads_validate_size_and_are_one_time() {
        let registry = FileUploadRegistry::default();
        let session = registry.new_session();
        let reservation = registry.reserve(&session, 3).unwrap();
        let token = registry.register_reserved(reservation);
        let mut upload = registry.begin(&token, Some(3)).await.unwrap();
        upload.write(Bytes::from_static(&[0, 255])).await.unwrap();
        upload.write(Bytes::from_static(&[128])).await.unwrap();
        upload.finish().unwrap();

        let file = registry.take_completed(&token).unwrap();
        assert_eq!(std::fs::read(&file.path).unwrap(), vec![0, 255, 128]);
        assert!(matches!(
            registry.begin(&token, Some(3)).await,
            Err(UploadError::UnknownOrExpired)
        ));
    }

    #[tokio::test]
    async fn completion_only_requires_its_own_http_upload_to_finish() {
        let registry = FileUploadRegistry::default();
        let session = registry.new_session();
        let reservation = registry.reserve(&session, 1).unwrap();
        let first_token = registry.register_reserved(reservation);
        let reservation = registry.reserve(&session, 0).unwrap();
        let second_token = registry.register_reserved(reservation);
        assert!(matches!(
            registry.take_completed(&first_token),
            Err(UploadError::Incomplete)
        ));
        let mut first = registry.begin(&first_token, Some(1)).await.unwrap();
        assert!(matches!(
            registry.take_completed(&first_token),
            Err(UploadError::Incomplete)
        ));
        first.write(Bytes::from_static(b"x")).await.unwrap();
        first.finish().unwrap();
        let file = registry.take_completed(&first_token).unwrap();
        assert_eq!(std::fs::read(&file.path).unwrap(), b"x");
        assert!(matches!(
            registry.take_completed(&second_token),
            Err(UploadError::Incomplete)
        ));
        registry
            .begin(&second_token, Some(0))
            .await
            .unwrap()
            .finish()
            .unwrap();
        assert!(
            registry
                .take_completed(&second_token)
                .unwrap()
                .path
                .exists()
        );
    }

    #[tokio::test]
    async fn an_invalid_file_does_not_cancel_another_upload() {
        let registry = FileUploadRegistry::default();
        let session = registry.new_session();
        let reservation = registry.reserve(&session, 3).unwrap();
        let first_token = registry.register_reserved(reservation);
        let reservation = registry.reserve(&session, 1).unwrap();
        let second_token = registry.register_reserved(reservation);
        let mut first = registry.begin(&first_token, None).await.unwrap();
        assert_eq!(
            first.write(Bytes::from_static(&[1, 2, 3, 4])).await,
            Err(UploadError::SizeMismatch)
        );
        drop(first);
        assert!(matches!(
            registry.take_completed(&first_token),
            Err(UploadError::Canceled)
        ));
        registry.wait_for_cleanup(&first_token).await;
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 1);
        let mut second = registry.begin(&second_token, Some(1)).await.unwrap();
        second.write(Bytes::from_static(b"x")).await.unwrap();
        second.finish().unwrap();
        drop(registry.take_completed(&second_token).unwrap());
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 0);
    }

    #[tokio::test]
    async fn cancel_stops_an_active_http_request() {
        let registry = FileUploadRegistry::default();
        let session = registry.new_session();
        let reservation = registry.reserve(&session, 3).unwrap();
        let token = registry.register_reserved(reservation);
        let mut upload = registry.begin(&token, Some(3)).await.unwrap();
        registry.cancel(&token);

        assert_eq!(
            upload.write(Bytes::from_static(&[1])).await,
            Err(UploadError::Canceled)
        );
        assert_eq!(upload.finish(), Err(UploadError::Canceled));
    }

    #[tokio::test]
    async fn quota_limit_covers_all_live_upload_files() {
        let registry = FileUploadRegistry::new(3);
        let session = registry.new_session();
        let reservation = registry.reserve(&session, 3).unwrap();
        let token = registry.register_reserved(reservation);
        let upload = registry.begin(&token, Some(3)).await.unwrap();
        registry.cancel(&token);

        assert_eq!(
            registry.reserve(&session, 1).err(),
            Some(UploadError::LimitExceeded)
        );
        drop(upload);
        assert!(registry.reserve(&session, 1).is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn expired_uploads_release_quota_before_registration() {
        let registry = FileUploadRegistry::new(4);
        let session = registry.new_session();
        let reservation = registry.reserve(&session, 4).unwrap();
        let expired = registry.register_reserved(reservation);
        tokio::time::advance(DEFAULT_UPLOAD_TIMEOUT + Duration::from_secs(1)).await;

        let reservation = registry.reserve(&session, 4).unwrap();
        let token = registry.register_reserved(reservation);
        assert_eq!(
            registry.begin(&expired, None).await.err(),
            Some(UploadError::UnknownOrExpired)
        );
        registry.cancel(&token);
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn unused_uploads_release_quota_at_expiration() {
        let registry = FileUploadRegistry::new(4);
        let session = registry.new_session();
        let reservation = registry.reserve(&session, 4).unwrap();
        let token = registry.register_reserved(reservation);
        let expiration = registry.wait_for_cleanup(&token);
        futures_util::pin_mut!(expiration);

        assert!(futures_util::poll!(&mut expiration).is_pending());
        tokio::time::advance(DEFAULT_UPLOAD_TIMEOUT - Duration::from_secs(1)).await;
        assert!(futures_util::poll!(&mut expiration).is_pending());
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 4);

        tokio::time::advance(Duration::from_secs(2)).await;
        assert!(futures_util::poll!(&mut expiration).is_ready());
        assert!(registry.uploads.lock().unwrap().is_empty());
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn active_uploads_do_not_extend_another_uploads_deadline() {
        let registry = FileUploadRegistry::new(4);
        let session = registry.new_session();
        let reservation = registry.reserve(&session, 3).unwrap();
        let first_token = registry.register_reserved(reservation);
        let reservation = registry.reserve(&session, 1).unwrap();
        let second_token = registry.register_reserved(reservation);
        let first_expiration = registry.wait_for_cleanup(&first_token);
        let second_expiration = registry.wait_for_cleanup(&second_token);
        futures_util::pin_mut!(first_expiration, second_expiration);
        assert!(futures_util::poll!(&mut first_expiration).is_pending());
        assert!(futures_util::poll!(&mut second_expiration).is_pending());
        let mut first = registry.begin(&first_token, Some(3)).await.unwrap();
        tokio::time::advance(DEFAULT_UPLOAD_TIMEOUT + Duration::from_secs(1)).await;
        assert!(futures_util::poll!(&mut first_expiration).is_pending());
        assert!(futures_util::poll!(&mut second_expiration).is_ready());
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 3);
        assert!(registry.reserve(&session, 1).is_ok());
        first
            .write(Bytes::from_static(&[0, 255, 128]))
            .await
            .unwrap();
        first.finish().unwrap();
        let file = registry.take_completed(&first_token).unwrap();
        assert_eq!(std::fs::read(&file.path).unwrap(), vec![0, 255, 128]);
        drop(file);
        assert_eq!(session.reserved_bytes.load(Ordering::Acquire), 0);
    }
}
