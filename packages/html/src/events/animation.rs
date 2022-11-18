use dioxus_core::UiEvent;

pub type AnimationEvent = UiEvent<AnimationData>;

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct AnimationData {
    pub animation_name: String,
    pub pseudo_element: String,
    pub elapsed_time: f32,
}
