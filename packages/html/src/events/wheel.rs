use dioxus_core::Event;
use std::fmt::Formatter;

use crate::geometry::WheelDelta;

pub type WheelEvent = Event<WheelData>;

pub struct WheelData {
    inner: Box<dyn HasWheelData>,
}

impl<E: HasWheelData> From<E> for WheelData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for WheelData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WheelData")
            .field("delta", &self.delta())
            .finish()
    }
}

impl PartialEq for WheelData {
    fn eq(&self, other: &Self) -> bool {
        self.inner.delta() == other.inner.delta()
    }
}

impl WheelData {
    /// The amount of wheel movement
    #[allow(deprecated)]
    pub fn delta(&self) -> WheelDelta {
        self.inner.delta()
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of WheelData
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializedWheelData {
    delta: WheelDelta,
}

#[cfg(feature = "serialize")]
impl From<&WheelData> for SerializedWheelData {
    fn from(data: &WheelData) -> Self {
        Self {
            delta: data.inner.delta(),
        }
    }
}

#[cfg(feature = "serialize")]
impl HasWheelData for SerializedWheelData {
    fn delta(&self) -> WheelDelta {
        self.delta
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for WheelData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedWheelData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for WheelData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedWheelData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

impl_event![
    WheelData;

    /// Called when the mouse wheel is rotated over an element.
    onwheel
];

pub trait HasWheelData: std::any::Any {
    /// The amount of wheel movement
    fn delta(&self) -> WheelDelta;

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}
