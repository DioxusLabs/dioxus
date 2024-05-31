use std::{
    fmt::{Display, Formatter},
    future::Future,
    pin::Pin,
};

pub trait ObserverEntryBacking: std::any::Any {
    /// Return self as Any
    fn as_any(&self) -> &dyn std::any::Any;

    /// Get the border box size of the observed element
    fn get_border_box_size(&self) -> Pin<Box<dyn Future<Output = ResizedResult<PixelsSize>>>> {
        Box::pin(async { Err(ResizedError::NotSupported) })
    }

    /// Get the content box size of the observed element
    fn get_content_box_size(&self) -> Pin<Box<dyn Future<Output = ResizedResult<PixelsSize>>>> {
        Box::pin(async { Err(ResizedError::NotSupported) })
    }

    /// Get the content box size in device pixels of the observed element
    fn get_device_content_box_size(
        &self,
    ) -> Pin<Box<dyn Future<Output = ResizedResult<PixelsRect>>>> {
        Box::pin(async { Err(ResizedError::NotSupported) })
    }
}

impl ObserverEntryBacking for () {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ResizedData {
    inner: Box<dyn ObserverEntryBacking>,
}

impl<E: ObserverEntryBacking> From<E> for ResizedData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for ResizedData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResizedData").finish()
    }
}

impl ResizedData {
    /// Create a new ResizedData
    pub fn new(inner: impl ObserverEntryBacking + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Get the border box size of the observed element
    pub async fn get_border_box_size(&self) -> ResizedResult<PixelsSize> {
        self.inner.get_border_box_size().await
    }

    /// Get the content box size of the observed element
    pub async fn get_content_box_size(&self) -> ResizedResult<PixelsSize> {
        self.inner.get_content_box_size().await
    }

    /// Get the content box size in device pixels of the observed element
    pub async fn get_device_content_box_size(&self) -> ResizedResult<PixelsRect> {
        self.inner.get_device_content_box_size().await
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

use dioxus_core::Event;

use crate::geometry::{PixelsRect, PixelsSize};

pub type ResizedEvent = Event<ResizedData>;

impl_event! {
    ResizedData;

    /// onresized
    onresized
}

/// The ResizedResult type for the ResizedData
pub type ResizedResult<T> = Result<T, ResizedError>;

#[derive(Debug)]
/// The error type for the MountedData
#[non_exhaustive]
pub enum ResizedError {
    /// The renderer does not support the requested operation
    NotSupported,
    /// The element was not found
    OperationFailed(Box<dyn std::error::Error>),
}

impl Display for ResizedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResizedError::NotSupported => {
                write!(f, "The renderer does not support the requested operation")
            }
            ResizedError::OperationFailed(e) => {
                write!(f, "The operation failed: {}", e)
            }
        }
    }
}

impl std::error::Error for ResizedError {}
