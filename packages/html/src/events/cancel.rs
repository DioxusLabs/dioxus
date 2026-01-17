use dioxus_core::Event;

pub type CancelEvent = Event<CancelData>;

pub struct CancelData {
    inner: Box<dyn HasCancelData>,
}

impl<E: HasCancelData> From<E> for CancelData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for CancelData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancelData").finish()
    }
}

impl PartialEq for CancelData {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl CancelData {
    /// Create a new CancelData
    pub fn new(inner: impl HasCancelData + 'static) -> Self {
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
/// A serialized version of CancelData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedCancelData {}

#[cfg(feature = "serialize")]
impl From<&CancelData> for SerializedCancelData {
    fn from(_: &CancelData) -> Self {
        Self {}
    }
}

#[cfg(feature = "serialize")]
impl HasCancelData for SerializedCancelData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for CancelData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedCancelData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for CancelData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedCancelData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

pub trait HasCancelData: std::any::Any {
    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

impl_event! {
    CancelData;

    /// oncancel
    oncancel
}
