use dioxus_core_types::Event;

pub type ToggleEvent = Event<ToggleData>;

pub struct ToggleData {
    inner: Box<dyn HasToggleData>,
}

impl<E: HasToggleData> From<E> for ToggleData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for ToggleData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToggleData").finish()
    }
}

impl PartialEq for ToggleData {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl ToggleData {
    /// Create a new ToggleData
    pub fn new(inner: impl HasToggleData + 'static) -> Self {
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

#[cfg(feature = "serialize")]
/// A serialized version of ToggleData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedToggleData {}

#[cfg(feature = "serialize")]
impl From<&ToggleData> for SerializedToggleData {
    fn from(_: &ToggleData) -> Self {
        Self {}
    }
}

#[cfg(feature = "serialize")]
impl HasToggleData for SerializedToggleData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for ToggleData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedToggleData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for ToggleData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedToggleData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

pub trait HasToggleData: std::any::Any {
    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

impl_event! {
    ToggleData;

    /// ontoggle
    ontoggle
}
