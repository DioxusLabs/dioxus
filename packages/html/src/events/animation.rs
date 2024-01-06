use dioxus_core::Event;

pub type AnimationEvent = Event<AnimationData>;

pub struct AnimationData {
    inner: Box<dyn HasAnimationData>,
}

impl AnimationData {
    /// Create a new AnimationData
    pub fn new(inner: impl HasAnimationData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// The name of the animation
    pub fn animation_name(&self) -> String {
        self.inner.animation_name()
    }

    /// The name of the pseudo-element the animation runs on
    pub fn pseudo_element(&self) -> String {
        self.inner.pseudo_element()
    }

    /// The amount of time the animation has been running
    pub fn elapsed_time(&self) -> f32 {
        self.inner.elapsed_time()
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_ref().as_any().downcast_ref::<T>()
    }
}

impl<E: HasAnimationData> From<E> for AnimationData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for AnimationData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationData")
            .field("animation_name", &self.animation_name())
            .field("pseudo_element", &self.pseudo_element())
            .field("elapsed_time", &self.elapsed_time())
            .finish()
    }
}

impl PartialEq for AnimationData {
    fn eq(&self, other: &Self) -> bool {
        self.animation_name() == other.animation_name()
            && self.pseudo_element() == other.pseudo_element()
            && self.elapsed_time() == other.elapsed_time()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of AnimationData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedAnimationData {
    animation_name: String,
    pseudo_element: String,
    elapsed_time: f32,
}

#[cfg(feature = "serialize")]
impl From<&AnimationData> for SerializedAnimationData {
    fn from(data: &AnimationData) -> Self {
        Self {
            animation_name: data.animation_name(),
            pseudo_element: data.pseudo_element(),
            elapsed_time: data.elapsed_time(),
        }
    }
}

#[cfg(feature = "serialize")]
impl HasAnimationData for SerializedAnimationData {
    fn animation_name(&self) -> String {
        self.animation_name.clone()
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
impl serde::Serialize for AnimationData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedAnimationData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for AnimationData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedAnimationData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

/// A trait for any object that has the data for an animation event
pub trait HasAnimationData: std::any::Any {
    /// The name of the animation
    fn animation_name(&self) -> String;

    /// The name of the pseudo-element the animation runs on
    fn pseudo_element(&self) -> String;

    /// The amount of time the animation has been running
    fn elapsed_time(&self) -> f32;

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

impl_event! [
    AnimationData;

    /// onanimationstart
    onanimationstart

    /// onanimationend
    onanimationend

    /// onanimationiteration
    onanimationiteration
];
