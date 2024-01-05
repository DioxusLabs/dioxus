use dioxus_core::Event;

pub type TransitionEvent = Event<TransitionData>;

pub struct TransitionData {
    inner: Box<dyn HasTransitionData>,
}

impl<E: HasTransitionData> From<E> for TransitionData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for TransitionData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionData")
            .field("property_name", &self.inner.property_name())
            .field("pseudo_element", &self.inner.pseudo_element())
            .field("elapsed_time", &self.inner.elapsed_time())
            .finish()
    }
}

impl PartialEq for TransitionData {
    fn eq(&self, other: &Self) -> bool {
        self.inner.property_name() == other.inner.property_name()
            && self.inner.pseudo_element() == other.inner.pseudo_element()
            && self.inner.elapsed_time() == other.inner.elapsed_time()
    }
}

impl TransitionData {
    /// Create a new TransitionData
    pub fn new(inner: impl HasTransitionData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of TransitionData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedTransitionData {
    property_name: String,
    pseudo_element: String,
    elapsed_time: f32,
}

#[cfg(feature = "serialize")]
impl From<&TransitionData> for SerializedTransitionData {
    fn from(data: &TransitionData) -> Self {
        Self {
            property_name: data.inner.property_name(),
            pseudo_element: data.inner.pseudo_element(),
            elapsed_time: data.inner.elapsed_time(),
        }
    }
}

#[cfg(feature = "serialize")]
impl HasTransitionData for SerializedTransitionData {
    fn property_name(&self) -> String {
        self.property_name.clone()
    }

    fn pseudo_element(&self) -> String {
        self.pseudo_element.clone()
    }

    fn elapsed_time(&self) -> f32 {
        self.elapsed_time
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for TransitionData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedTransitionData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for TransitionData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedTransitionData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

pub trait HasTransitionData: std::any::Any {
    fn property_name(&self) -> String;
    fn pseudo_element(&self) -> String;
    fn elapsed_time(&self) -> f32;
    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

impl_event! {
    TransitionData;

    /// transitionend
    ontransitionend
}
