use dioxus_core::Event;
use euclid::UnknownUnit;
use std::fmt::{Debug, Formatter};

use crate::geometry::{LinesVector, PagesVector, PixelsVector, WheelDelta};

pub type WheelEvent = Event<WheelData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, PartialEq, Default)]
pub struct WheelData {
    #[deprecated(since = "0.3.0", note = "use delta() instead")]
    pub delta_mode: u32,
    #[deprecated(since = "0.3.0", note = "use delta() instead")]
    pub delta_x: f64,
    #[deprecated(since = "0.3.0", note = "use delta() instead")]
    pub delta_y: f64,
    #[deprecated(since = "0.3.0", note = "use delta() instead")]
    pub delta_z: f64,
}

impl_event![
    WheelData;

    /// Called when the mouse wheel is rotated over an element.
    onwheel
];

impl WheelData {
    /// Construct a new WheelData with the specified wheel movement delta
    pub fn new(delta: WheelDelta) -> Self {
        let (delta_mode, vector) = match delta {
            WheelDelta::Pixels(v) => (0, v.cast_unit::<UnknownUnit>()),
            WheelDelta::Lines(v) => (1, v.cast_unit::<UnknownUnit>()),
            WheelDelta::Pages(v) => (2, v.cast_unit::<UnknownUnit>()),
        };

        #[allow(deprecated)]
        WheelData {
            delta_mode,
            delta_x: vector.x,
            delta_y: vector.y,
            delta_z: vector.z,
        }
    }

    /// Construct from the attributes of the web wheel event
    pub fn from_web_attributes(delta_mode: u32, delta_x: f64, delta_y: f64, delta_z: f64) -> Self {
        #[allow(deprecated)]
        Self {
            delta_mode,
            delta_x,
            delta_y,
            delta_z,
        }
    }

    /// The amount of wheel movement
    #[allow(deprecated)]
    pub fn delta(&self) -> WheelDelta {
        let x = self.delta_x;
        let y = self.delta_y;
        let z = self.delta_z;
        match self.delta_mode {
            0 => WheelDelta::Pixels(PixelsVector::new(x, y, z)),
            1 => WheelDelta::Lines(LinesVector::new(x, y, z)),
            2 => WheelDelta::Pages(PagesVector::new(x, y, z)),
            _ => panic!("Invalid delta mode, {:?}", self.delta_mode),
        }
    }
}

impl Debug for WheelData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WheelData")
            .field("delta", &self.delta())
            .finish()
    }
}
