use dioxus_core_types::Event;

pub type ScrollEvent = Event<ScrollData>;

pub struct ScrollData {
    inner: Box<dyn HasScrollData>,
}

impl<E: HasScrollData> From<E> for ScrollData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl ScrollData {
    /// Create a new ScrollData
    pub fn new(inner: impl HasScrollData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Downcast this event to a concrete event type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

impl std::fmt::Debug for ScrollData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScrollData").finish()
    }
}

impl PartialEq for ScrollData {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of ScrollData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedScrollData {}

#[cfg(feature = "serialize")]
impl From<&ScrollData> for SerializedScrollData {
    fn from(_: &ScrollData) -> Self {
        Self {}
    }
}

#[cfg(feature = "serialize")]
impl HasScrollData for SerializedScrollData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for ScrollData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedScrollData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for ScrollData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedScrollData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

pub trait HasScrollData: std::any::Any {
    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

impl_event! {
    ScrollData;

    /// onscroll
    onscroll
}
