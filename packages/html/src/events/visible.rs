use std::{
    fmt::{Display, Formatter},
    time::SystemTime,
};

pub struct VisibleData {
    inner: Box<dyn HasVisibleData>,
}

impl<E: HasVisibleData> From<E> for VisibleData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl VisibleData {
    /// Create a new VisibleData
    pub fn new(inner: impl HasVisibleData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Get the bounds rectangle of the target element
    pub fn get_bounding_client_rect(&self) -> VisibleResult<PixelsRect> {
        self.inner.get_bounding_client_rect()
    }

    /// Get the ratio of the intersectionRect to the boundingClientRect
    pub fn get_intersection_ratio(&self) -> VisibleResult<f64> {
        self.inner.get_intersection_ratio()
    }

    /// Get the rect representing the target's visible area
    pub fn get_intersection_rect(&self) -> VisibleResult<PixelsRect> {
        self.inner.get_intersection_rect()
    }

    /// Get if the target element intersects with the intersection observer's root
    pub fn is_intersecting(&self) -> VisibleResult<bool> {
        self.inner.is_intersecting()
    }

    /// Get the rect for the intersection observer's root
    pub fn get_root_bounds(&self) -> VisibleResult<PixelsRect> {
        self.inner.get_root_bounds()
    }

    /// Get a timestamp indicating the time at which the intersection was recorded
    pub fn get_time(&self) -> VisibleResult<SystemTime> {
        self.inner.get_time()
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

impl std::fmt::Debug for VisibleData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisibleData")
            .field(
                "bounding_client_rect",
                &self.inner.get_bounding_client_rect(),
            )
            .field("intersection_ratio", &self.inner.get_intersection_ratio())
            .field("intersection_rect", &self.inner.get_intersection_rect())
            .field("is_intersecting", &self.inner.is_intersecting())
            .field("root_bounds", &self.inner.get_root_bounds())
            .field("time", &self.inner.get_time())
            .finish()
    }
}

impl PartialEq for VisibleData {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[cfg(feature = "serialize")]
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct DOMRect {
    bottom: f64, // The bottom coordinate value of the DOMRect (usually the same as y + height)
    height: f64, // The height of the DOMRect
    left: f64,   // The left coordinate value of the DOMRect (usually the same as x)
    right: f64,  // The right coordinate value of the DOMRect (usually the same as x + width)
    top: f64,    // The top coordinate value of the DOMRect (usually the same as y)
    width: f64,  // The width of the DOMRect
    x: f64,      // The x coordinate of the DOMRect's origin
    y: f64,      // The y coordinate of the DOMRect's origin
}

#[cfg(feature = "serialize")]
impl From<PixelsRect> for DOMRect {
    fn from(rect: PixelsRect) -> Self {
        let x = rect.origin.x;
        let y = rect.origin.y;
        let height = rect.height();
        let width = rect.width();

        Self {
            bottom: y + height,
            height,
            left: x,
            right: x + width,
            top: y,
            width,
            x,
            y,
        }
    }
}

#[cfg(feature = "serialize")]
impl From<&DOMRect> for PixelsRect {
    fn from(rect: &DOMRect) -> Self {
        PixelsRect::new(
            euclid::Point2D::new(rect.x, rect.y),
            euclid::Size2D::new(rect.width, rect.height),
        )
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of VisibleData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedVisibleData {
    pub bounding_client_rect: DOMRect,
    pub intersection_ratio: f64,
    pub intersection_rect: DOMRect,
    pub is_intersecting: bool,
    pub root_bounds: DOMRect,
    pub time_ms: u128,
}

#[cfg(feature = "serialize")]
impl SerializedVisibleData {
    /// Create a new SerializedVisibleData
    pub fn new(
        bounding_client_rect: DOMRect,
        intersection_ratio: f64,
        intersection_rect: DOMRect,
        is_intersecting: bool,
        root_bounds: DOMRect,
        time_ms: u128,
    ) -> Self {
        Self {
            bounding_client_rect,
            intersection_ratio,
            intersection_rect,
            is_intersecting,
            root_bounds,
            time_ms,
        }
    }
}

#[cfg(feature = "serialize")]
impl From<&VisibleData> for SerializedVisibleData {
    fn from(data: &VisibleData) -> Self {
        Self::new(
            data.get_bounding_client_rect().unwrap().into(),
            data.get_intersection_ratio().unwrap(),
            data.get_intersection_rect().unwrap().into(),
            data.is_intersecting().unwrap(),
            data.get_root_bounds().unwrap().into(),
            data.get_time().unwrap().elapsed().unwrap().as_millis(),
        )
    }
}

#[cfg(feature = "serialize")]
impl HasVisibleData for SerializedVisibleData {
    /// Get the bounds rectangle of the target element
    fn get_bounding_client_rect(&self) -> VisibleResult<PixelsRect> {
        Ok((&self.bounding_client_rect).into())
    }

    /// Get the ratio of the intersectionRect to the boundingClientRect
    fn get_intersection_ratio(&self) -> VisibleResult<f64> {
        Ok(self.intersection_ratio)
    }

    /// Get the rect representing the target's visible area
    fn get_intersection_rect(&self) -> VisibleResult<PixelsRect> {
        Ok((&self.intersection_rect).into())
    }

    /// Get if the target element intersects with the intersection observer's root
    fn is_intersecting(&self) -> VisibleResult<bool> {
        Ok(self.is_intersecting)
    }

    /// Get the rect for the intersection observer's root
    fn get_root_bounds(&self) -> VisibleResult<PixelsRect> {
        Ok((&self.root_bounds).into())
    }

    /// Get a timestamp indicating the time at which the intersection was recorded
    fn get_time(&self) -> VisibleResult<SystemTime> {
        Ok(SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(self.time_ms as u64))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for VisibleData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedVisibleData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for VisibleData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedVisibleData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

pub trait HasVisibleData: std::any::Any {
    /// Get the bounds rectangle of the target element
    fn get_bounding_client_rect(&self) -> VisibleResult<PixelsRect> {
        Err(VisibleError::NotSupported)
    }

    /// Get the ratio of the intersectionRect to the boundingClientRect
    fn get_intersection_ratio(&self) -> VisibleResult<f64> {
        Err(VisibleError::NotSupported)
    }

    /// Get the rect representing the target's visible area
    fn get_intersection_rect(&self) -> VisibleResult<PixelsRect> {
        Err(VisibleError::NotSupported)
    }

    /// Get if the target element intersects with the intersection observer's root
    fn is_intersecting(&self) -> VisibleResult<bool> {
        Err(VisibleError::NotSupported)
    }

    /// Get the rect for the intersection observer's root
    fn get_root_bounds(&self) -> VisibleResult<PixelsRect> {
        Err(VisibleError::NotSupported)
    }

    /// Get a timestamp indicating the time at which the intersection was recorded
    fn get_time(&self) -> VisibleResult<SystemTime> {
        Err(VisibleError::NotSupported)
    }

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

use dioxus_core::Event;

use crate::geometry::PixelsRect;

pub type VisibleEvent = Event<VisibleData>;

impl_event! {
    VisibleData;

    /// onvisible
    onvisible
}

/// The VisibleResult type for the VisibleData
pub type VisibleResult<T> = Result<T, VisibleError>;

#[derive(Debug)]
/// The error type for the VisibleData
#[non_exhaustive]
pub enum VisibleError {
    /// The renderer does not support the requested operation
    NotSupported,
    /// The element was not found
    OperationFailed(Box<dyn std::error::Error>),
    /// The target element had no associated ElementId
    NoElementId,
}

impl Display for VisibleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VisibleError::NotSupported => {
                write!(f, "The renderer does not support the requested operation")
            }
            VisibleError::OperationFailed(e) => {
                write!(f, "The operation failed: {}", e)
            }
            VisibleError::NoElementId => {
                write!(f, "The target had no associated ElementId")
            }
        }
    }
}

impl std::error::Error for VisibleError {}
