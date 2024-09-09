use dioxus::prelude::*;
use dioxus_core::DynamicNode;
use dioxus_material::use_theme;
use serde::{Deserialize, Serialize};

/// A controllable property.
pub trait Control: Sized {
    type State;

    /// Create the initial state.
    fn state(default: Option<impl Into<Self>>) -> Self::State;

    /// Convert the current state to `Self`.
    fn from_state(state: &Self::State) -> Self;

    /// Render the controller element.
    fn control(name: &'static str, state: Signal<Self::State>) -> Element;
}

impl Control for String {
    type State = String;

    fn state(default: Option<impl Into<Self>>) -> Self::State {
        default
            .map(Into::into)
            .map(String::from)
            .unwrap_or_default()
    }

    fn from_state(state: &Self::State) -> Self {
        state.clone()
    }

    fn control(_name: &'static str, mut state: Signal<Self::State>) -> Element {
        rsx!(Input {
            value: state,
            oninput: move |event: FormEvent| state.set(event.data.value())
        })
    }
}

#[derive(Default)]
pub struct Json<T>(pub T);

impl<T> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> IntoDynNode for Json<T>
where
    T: Clone + Default + for<'de> Deserialize<'de> + Serialize,
{
    fn into_dyn_node(self) -> DynamicNode {
        let s = serde_json::to_string(&self.0).unwrap();
        DynamicNode::make_node(s)
    }
}

impl<T> Control for Json<T>
where
    T: Clone + Default + for<'de> Deserialize<'de> + Serialize,
{
    type State = T;

    fn state(default: Option<impl Into<Self>>) -> Self::State {
        default.map(Into::into).unwrap_or_default().0
    }

    fn from_state(state: &Self::State) -> Self {
        Self(state.clone())
    }

    fn control(_name: &'static str, mut state: Signal<Self::State>) -> Element {
        let json = serde_json::to_string(&*state.read()).unwrap();

        rsx!(Input {
            value: "{json}",
            oninput: move |event: FormEvent| {
                let value = event.data.value();
                if let Ok(new_state) = serde_json::from_str(&value) {
                    state.set(new_state);
                }
            }
        })
    }
}

#[component]
fn Input(value: String, oninput: EventHandler<FormEvent>) -> Element {
    let theme = use_theme();

    rsx!(input {
        border: "2px solid #e7e7e7",
        padding: "10px",
        border_radius: &*theme.border_radius_small,
        font_size: "{theme.label_small}px",
        outline: "none",
        background: "none",
        value: value,
        oninput: move |event| oninput.call(event)
    })
}
