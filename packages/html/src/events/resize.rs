use std::fmt::{Display, Formatter};

pub trait ObserverEntryBacking: std::any::Any {
    /// Return self as Any
    fn as_any(&self) -> &dyn std::any::Any;

    /// Get the border box size of the observed element
    fn get_border_box_size(&self) -> ResizeResult<Vec<PixelsSize>> {
        Err(ResizeError::NotSupported)
    }

    /// Get the content box size of the observed element
    fn get_content_box_size(&self) -> ResizeResult<Vec<PixelsSize>> {
        Err(ResizeError::NotSupported)
    }
}

pub struct ResizeData {
    inner: Box<dyn ObserverEntryBacking>,
}

impl<E: ObserverEntryBacking> From<E> for ResizeData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for ResizeData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResizeData")
            .field("border_box_size", &self.inner.get_border_box_size())
            .field("content_box_size", &self.inner.get_content_box_size())
            .finish()
    }
}

impl ResizeData {
    /// Create a new ResizeData
    pub fn new(inner: impl ObserverEntryBacking + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Get the border box size of the observed element
    pub fn get_border_box_size(&self) -> ResizeResult<Vec<PixelsSize>> {
        self.inner.get_border_box_size()
    }

    /// Get the content box size of the observed element
    pub fn get_content_box_size(&self) -> ResizeResult<Vec<PixelsSize>> {
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
/// A serialized version of ResizeData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedResizeData {
    border_box_size: Vec<SerializedResizeObserverSize>,
    content_box_size: Vec<SerializedResizeObserverSize>,
}

#[cfg(feature = "serialize")]
impl From<&ResizeData> for SerializedResizeData {
    fn from(data: &ResizeData) -> Self {
        let mut border_box_sizes = Vec::new();
        if let Ok(sizes) = data.inner.get_border_box_size() {
            for size in sizes {
                border_box_sizes.push(SerializedResizeObserverSize {
                    // block_size matchs the height of the element if its writing-mode is horizontal, the width otherwise
                    block_size: size.height,
                    // inline_size matchs the width of the element if its writing-mode is horizontal, the height otherwise
                    inline_size: size.width,
                });
            }
        }

        let mut content_box_sizes = Vec::new();
        if let Ok(sizes) = data.inner.get_content_box_size() {
            for size in sizes {
                content_box_sizes.push(SerializedResizeObserverSize {
                    // block_size matchs the height of the element if its writing-mode is horizontal, the width otherwise
                    block_size: size.height,
                    // inline_size matchs the width of the element if its writing-mode is horizontal, the height otherwise
                    inline_size: size.width,
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
        fn $meth_name(&self) -> ResizeResult<Vec<PixelsSize>> {
            if self.$field_name.len() > 0 {
                let sizes = self
                    .$field_name
                    .iter()
                    .map(|s| PixelsSize::new(s.inline_size, s.block_size))
                    .collect();
                Ok(sizes)
            } else {
                Err(ResizeError::NotSupported)
            }
        }
    };
}

#[cfg(feature = "serialize")]
impl ObserverEntryBacking for SerializedResizeData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    get_box_size!(get_border_box_size, border_box_size);

    get_box_size!(get_content_box_size, content_box_size);
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

use dioxus_core::Event;

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
