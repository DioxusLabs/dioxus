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
    /// Create a new WheelData
    pub fn new(inner: impl HasWheelData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

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
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedWheelData {
    pub delta_mode: u32,
    pub delta_x: f64,
    pub delta_y: f64,
    pub delta_z: f64,
}

#[cfg(feature = "serialize")]
impl SerializedWheelData {
    /// Create a new SerializedWheelData
    pub fn new(delta: WheelDelta) -> Self {
        let delta_mode = match delta {
            WheelDelta::Pixels(_) => 0,
            WheelDelta::Lines(_) => 1,
            WheelDelta::Pages(_) => 2,
        };
        let delta_raw = delta.strip_units();
        Self {
            delta_mode,
            delta_x: delta_raw.x,
            delta_y: delta_raw.y,
            delta_z: delta_raw.z,
        }
    }
}

#[cfg(feature = "serialize")]
impl From<&WheelData> for SerializedWheelData {
    fn from(data: &WheelData) -> Self {
        Self::new(data.delta())
    }
}

#[cfg(feature = "serialize")]
impl HasWheelData for SerializedWheelData {
    fn delta(&self) -> WheelDelta {
        WheelDelta::from_web_attributes(self.delta_mode, self.delta_x, self.delta_y, self.delta_z)
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
