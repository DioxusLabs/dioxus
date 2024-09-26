use std::fmt::{Display, Formatter};

pub struct ResizeData {
    inner: Box<dyn HasResizeData>,
}

impl<E: HasResizeData> From<E> for ResizeData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl ResizeData {
    /// Create a new ResizeData
    pub fn new(inner: impl HasResizeData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Get the border box size of the observed element
    pub fn get_border_box_size(&self) -> ResizeResult<PixelsSize> {
        self.inner.get_border_box_size()
    }

    /// Get the content box size of the observed element
    pub fn get_content_box_size(&self) -> ResizeResult<PixelsSize> {
        self.inner.get_content_box_size()
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

impl std::fmt::Debug for ResizeData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResizeData")
            .field("border_box_size", &self.inner.get_border_box_size())
            .field("content_box_size", &self.inner.get_content_box_size())
            .finish()
    }
}

impl PartialEq for ResizeData {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of ResizeData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedResizeData {
    pub border_box_size: PixelsSize,
    pub content_box_size: PixelsSize,
}

#[cfg(feature = "serialize")]
impl SerializedResizeData {
    /// Create a new SerializedResizeData
    pub fn new(border_box_size: PixelsSize, content_box_size: PixelsSize) -> Self {
        Self {
            border_box_size,
            content_box_size,
        }
    }
}

#[cfg(feature = "serialize")]
impl From<&ResizeData> for SerializedResizeData {
    fn from(data: &ResizeData) -> Self {
        Self::new(
            data.get_border_box_size().unwrap(),
            data.get_content_box_size().unwrap(),
        )
    }
}

#[cfg(feature = "serialize")]
impl HasResizeData for SerializedResizeData {
    /// Get the border box size of the observed element
    fn get_border_box_size(&self) -> ResizeResult<PixelsSize> {
        Ok(self.border_box_size)
    }

    /// Get the content box size of the observed element
    fn get_content_box_size(&self) -> ResizeResult<PixelsSize> {
        Ok(self.content_box_size)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for ResizeData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedResizeData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for ResizeData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedResizeData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

pub trait HasResizeData: std::any::Any {
    /// Get the border box size of the observed element
    fn get_border_box_size(&self) -> ResizeResult<PixelsSize> {
        Err(ResizeError::NotSupported)
    }
    /// Get the content box size of the observed element
    fn get_content_box_size(&self) -> ResizeResult<PixelsSize> {
        Err(ResizeError::NotSupported)
    }

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

use dioxus_core_types::Event;

use crate::geometry::PixelsSize;

pub type ResizeEvent = Event<ResizeData>;

impl_event! {
    ResizeData;

    /// onresize
    onresize
}

/// The ResizeResult type for the ResizeData
pub type ResizeResult<T> = Result<T, ResizeError>;

#[derive(Debug)]
/// The error type for the MountedData
#[non_exhaustive]
pub enum ResizeError {
    /// The renderer does not support the requested operation
    NotSupported,
    /// The element was not found
    OperationFailed(Box<dyn std::error::Error>),
}

impl Display for ResizeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResizeError::NotSupported => {
                write!(f, "The renderer does not support the requested operation")
            }
            ResizeError::OperationFailed(e) => {
                write!(f, "The operation failed: {}", e)
            }
        }
    }
}

impl std::error::Error for ResizeError {}
