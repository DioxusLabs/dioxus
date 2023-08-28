use dioxus_core::Event;

pub type TouchEvent = Event<TouchData>;
pub struct TouchData {
    inner: Box<dyn HasTouchData>,
}

impl<E: HasTouchData> From<E> for TouchData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for TouchData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TouchData")
            .field("alt_key", &self.alt_key())
            .field("ctrl_key", &self.ctrl_key())
            .field("meta_key", &self.meta_key())
            .field("shift_key", &self.shift_key())
            .finish()
    }
}

impl PartialEq for TouchData {
    fn eq(&self, other: &Self) -> bool {
        self.alt_key() == other.alt_key()
            && self.ctrl_key() == other.ctrl_key()
            && self.meta_key() == other.meta_key()
            && self.shift_key() == other.shift_key()
    }
}

impl TouchData {
    /// Returns true if the "ALT" key was down when the touch event was fired.
    pub fn alt_key(&self) -> bool {
        self.inner.alt_key()
    }

    /// Returns true if the "CTRL" key was down when the touch event was fired.
    pub fn ctrl_key(&self) -> bool {
        self.inner.ctrl_key()
    }

    /// Returns true if the "META" key was down when the touch event was fired.
    pub fn meta_key(&self) -> bool {
        self.inner.meta_key()
    }

    /// Returns true if the "SHIFT" key was down when the touch event was fired.
    pub fn shift_key(&self) -> bool {
        self.inner.shift_key()
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of TouchData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedTouchData {
    alt_key: bool,
    ctrl_key: bool,
    meta_key: bool,
    shift_key: bool,
}

#[cfg(feature = "serialize")]
impl From<&TouchData> for SerializedTouchData {
    fn from(data: &TouchData) -> Self {
        Self {
            alt_key: data.alt_key(),
            ctrl_key: data.ctrl_key(),
            meta_key: data.meta_key(),
            shift_key: data.shift_key(),
        }
    }
}

#[cfg(feature = "serialize")]
impl HasTouchData for SerializedTouchData {
    fn alt_key(&self) -> bool {
        self.alt_key
    }

    fn ctrl_key(&self) -> bool {
        self.ctrl_key
    }

    fn meta_key(&self) -> bool {
        self.meta_key
    }

    fn shift_key(&self) -> bool {
        self.shift_key
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for TouchData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedTouchData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for TouchData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedTouchData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

pub trait HasTouchData: std::any::Any {
    /// Returns true if the "ALT" key was down when the touch event was fired.
    fn alt_key(&self) -> bool;

    /// Returns true if the "CTRL" key was down when the touch event was fired.
    fn ctrl_key(&self) -> bool;

    /// Returns true if the "META" key was down when the touch event was fired.
    fn meta_key(&self) -> bool;

    /// Returns true if the "SHIFT" key was down when the touch event was fired.
    fn shift_key(&self) -> bool;

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

impl_event! {
    TouchData;
    /// touchstart
    ontouchstart

    /// touchmove
    ontouchmove

    /// touchend
    ontouchend

    /// touchcancel
    ontouchcancel
}
