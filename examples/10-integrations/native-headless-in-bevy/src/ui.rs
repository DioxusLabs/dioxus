use async_std::task::sleep;
use crossbeam_channel::{Receiver, Sender};
use dioxus::prelude::*;
use paste::paste;

macro_rules! define_ui_state {
    (
        $($field:ident : $type:ty = $default:expr),* $(,)?
    ) => { paste! {
        #[allow(dead_code)]
        #[derive(Clone, Copy)]
        pub struct UiState {
            $($field: Signal<$type>,)*
        }

        #[allow(dead_code)]
        impl UiState {
            fn default() -> Self {
                Self {
                    $($field: Signal::new($default),)*
                }
            }

            $(pub const [<DEFAULT_ $field:upper>]: $type = $default;)*
        }

        #[allow(dead_code)]
        pub enum UIMessage {
            $([<$field:camel>]($type),)*
        }
    }};
}

define_ui_state! {
    cube_color: [f32; 4] = [0.0, 0.0, 1.0, 1.0],
    cube_translation_speed: f32 = 2.0,
    cube_rotation_speed: f32 = 1.0,
    fps: f32 = 0.0,
}

#[derive(Clone)]
pub struct UIProps {
    pub ui_sender: Sender<UIMessage>,
    pub app_receiver: Receiver<UIMessage>,
}

pub fn ui(props: UIProps) -> Element {
    let mut state = use_context_provider(UiState::default);

    use_effect({
        let ui_sender = props.ui_sender.clone();
        move || {
            println!("Color changed to {:?}", state.cube_color);
            ui_sender
                .send(UIMessage::CubeColor((state.cube_color)()))
                .unwrap();
        }
    });

    use_effect({
        let ui_sender = props.ui_sender.clone();
        move || {
            println!("Rotation speed changed to {:?}", state.cube_rotation_speed);
            ui_sender
                .send(UIMessage::CubeRotationSpeed((state.cube_rotation_speed)()))
                .unwrap();
        }
    });

    use_effect({
        let ui_sender = props.ui_sender.clone();
        move || {
            println!(
                "Translation speed changed to {:?}",
                state.cube_translation_speed
            );
            ui_sender
                .send(UIMessage::CubeTranslationSpeed((state
                    .cube_translation_speed)(
                )))
                .unwrap();
        }
    });

    use_future(move || {
        let app_receiver = props.app_receiver.clone();
        async move {
            loop {
                // Update UI every 1s in this demo.
                sleep(std::time::Duration::from_millis(1000)).await;

                let mut fps = Option::<f32>::None;

                while let Ok(message) = app_receiver.try_recv() {
                    if let UIMessage::Fps(v) = message {
                        fps = Some(v)
                    }
                }

                if let Some(fps) = fps {
                    state.fps.set(fps);
                }
            }
        }
    });

    let color = *state.cube_color.read();
    let [r, g, b, a] = color.map(|c| (c * 255.0) as u8);

    println!("rgba({r}, {g}, {b}, {a})");

    rsx! {
        document::Stylesheet { href: asset!("/src/ui.css") }
        div {
            id: "panel",
            class: "catch-events",
            div {
                id: "title",
                h1 { "Dioxus In Bevy Example" }
            }
            div {
                id: "buttons",
                button {
                    id: "button-red",
                    class: "color-button",
                    onclick: move |_| state.cube_color.set([1.0, 0.0, 0.0, 1.0]),
                }
                button {
                    id: "button-green",
                    class: "color-button",
                    onclick: move |_| state.cube_color.set([0.0, 1.0, 0.0, 1.0]),
                }
                button {
                    id: "button-blue",
                    class: "color-button",
                    onclick: move |_| state.cube_color.set([0.0, 0.0, 1.0, 1.0]),
                }
            }
            div {
                id: "translation-speed-control",
                label { "Translation Speed:" }
                input {
                    r#type: "number",
                    min: "0.0",
                    max: "10.0",
                    step: "0.1",
                    value: "{state.cube_translation_speed}",
                    oninput: move |event| {
                        if let Ok(speed) = event.value().parse::<f32>() {
                            state.cube_translation_speed.set(speed);
                        }
                    }
                }
            }
            div {
                id: "rotation-speed-control",
                label { "Rotation Speed:" }
                input {
                    r#type: "number",
                    min: "0.0",
                    max: "10.0",
                    step: "0.1",
                    value: "{state.cube_rotation_speed}",
                    oninput: move |event| {
                        if let Ok(speed) = event.value().parse::<f32>() {
                            state.cube_rotation_speed.set(speed);
                        }
                    }
                }
            }
            div {
                flex: "0 0 150px",
                display: "grid",
                align_items: "center",
                justify_items: "center",
                div {
                    class: "spin-box",
                    background: "rgba({r}, {g}, {b}, {a}",
                }
            }
            div {
                id: "footer",
                p { "Bevy framerate: {state.fps}" }
            }
        }
    }
}
