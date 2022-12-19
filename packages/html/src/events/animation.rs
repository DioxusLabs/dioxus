use dioxus_core::Event;

pub type AnimationEvent = Event<AnimationData>;

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct AnimationData {
    pub animation_name: String,
    pub pseudo_element: String,
    pub elapsed_time: f32,
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
