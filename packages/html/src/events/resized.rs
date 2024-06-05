use std::fmt::{Display, Formatter};

pub trait ObserverEntryBacking: std::any::Any {
    /// Return self as Any
    fn as_any(&self) -> &dyn std::any::Any;

    /// Get the border box size of the observed element
    fn get_border_box_size(&self) -> ResizedResult<Vec<PixelsSize>> {
        Err(ResizedError::NotSupported)
    }

    /// Get the content box size of the observed element
    fn get_content_box_size(&self) -> ResizedResult<Vec<PixelsSize>> {
        Err(ResizedError::NotSupported)
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
        f.debug_struct("ResizedData")
            .field("border_box_size", &self.inner.get_border_box_size())
            .field("content_box_size", &self.inner.get_content_box_size())
            .finish()
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
    pub fn get_border_box_size(&self) -> ResizedResult<Vec<PixelsSize>> {
        self.inner.get_border_box_size()
    }

    /// Get the content box size of the observed element
    pub fn get_content_box_size(&self) -> ResizedResult<Vec<PixelsSize>> {
        self.inner.get_content_box_size()
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
struct SerializedResizeObserverSize {
    block_size: f64,
    inline_size: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
struct DomRect {
    bottom: f64,
    height: f64,
    left: f64,
    right: f64,
    top: f64,
    width: f64,
    x: f64,
    y: f64,
}

#[cfg(feature = "serialize")]
/// A serialized version of ResizedData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedResizedData {
    border_box_size: Vec<SerializedResizeObserverSize>,
    content_box_size: Vec<SerializedResizeObserverSize>,
}

#[cfg(feature = "serialize")]
impl From<&ResizedData> for SerializedResizedData {
    fn from(data: &ResizedData) -> Self {
        let mut border_box_sizes = Vec::new();
        if let Some(sizes) = data.inner.get_border_box_size().ok() {
            for size in sizes {
                border_box_sizes.push(SerializedResizeObserverSize {
                    block_size: size.width,
                    inline_size: size.height,
                });
            }
        }

        let mut content_box_sizes = Vec::new();
        if let Some(sizes) = data.inner.get_content_box_size().ok() {
            for size in sizes {
                content_box_sizes.push(SerializedResizeObserverSize {
                    block_size: size.width,
                    inline_size: size.height,
                });
            }
        }

        Self {
            border_box_size: border_box_sizes,
            content_box_size: content_box_sizes,
        }
    }
}

macro_rules! get_box_size {
    ($meth_name:ident, $field_name:ident) => {
        fn $meth_name(&self) -> ResizedResult<Vec<PixelsSize>> {
            if self.$field_name.len() > 0 {
                let sizes = self
                    .$field_name
                    .iter()
                    .map(|s| PixelsSize::new(s.block_size, s.inline_size))
                    .collect();
                Ok(sizes)
            } else {
                Err(ResizedError::NotSupported)
            }
        }
    };
}

#[cfg(feature = "serialize")]
impl ObserverEntryBacking for SerializedResizedData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    get_box_size!(get_border_box_size, border_box_size);

    get_box_size!(get_content_box_size, content_box_size);
}

#[cfg(feature = "serialize")]
impl serde::Serialize for ResizedData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedResizedData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for ResizedData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedResizedData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

use dioxus_core::Event;

use crate::geometry::PixelsSize;

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
