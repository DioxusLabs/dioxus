use bevy::{
    app::App,
    ecs::event::{EventReader, EventWriter},
    input::keyboard::KeyboardInput,
    log::{info, LogPlugin},
    window::{ReceivedCharacter, WindowDescriptor},
};
use dioxus::prelude::*;

#[derive(Debug, Clone)]
enum CoreCommand {
    Test,
}

#[derive(Debug, Clone)]
enum UICommand {
    Test,
    KeyboardInput(KeyboardInput),
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Bevy Dioxus Plugin Demo".to_string(),
            ..Default::default()
        })
        .add_plugin(DioxusDesktopPlugin::<CoreCommand, UICommand>::new(app, ()))
        .add_plugin(LogPlugin)
        .add_system(handle_core_command)
        .add_system(send_keyboard_input)
        .add_system(log_keyboard_event)
        .run();
}

fn handle_core_command(mut events: EventReader<CoreCommand>, mut ui: EventWriter<UICommand>) {
    for cmd in events.iter() {
        info!("🧠 {:?}", cmd);

        match cmd {
            CoreCommand::Test => ui.send(UICommand::Test),
        }
    }
}

fn send_keyboard_input(mut events: EventReader<KeyboardInput>, mut event: EventWriter<UICommand>) {
    for input in events.iter() {
        event.send(UICommand::KeyboardInput(input.clone()));
    }
}

fn log_keyboard_event(
    mut keyboard_input_events: EventReader<KeyboardInput>,
    mut received_character_events: EventReader<ReceivedCharacter>,
) {
    for input in keyboard_input_events.iter() {
        info!("🧠 {:?}", input.clone());
    }

    for received_char in received_character_events.iter() {
        info!("🧠 {:?}", received_char.clone());
    }
}

fn app(cx: Scope) -> Element {
    let window = use_bevy_window::<CoreCommand, UICommand>(&cx);
    let input = use_state(&cx, || None);
    let state = use_state(&cx, || None);

    use_future(&cx, (), |_| {
        let input = input.clone();
        let state = state.clone();
        let rx = window.receiver();

        async move {
            while let Some(cmd) = rx.receive().await {
                match cmd {
                    UICommand::KeyboardInput(i) => {
                        input.set(i.key_code);
                        state.set(Some(i.state));
                    }
                    _ => {}
                }
            }
        }
    });

    cx.render(rsx! (
        div {
            div {
                h1 { "Bevy Dioxus Plugin Example" },
                button {
                    onclick: |_e| window.send(CoreCommand::Test),
                    "Test",
                }
                button {
                    onclick: |_e| window.close(),
                    "Close",
                }
                button {
                    onclick: move |_| window.set_minimized(true),
                    "Minimize"
                }
            }

            input.and_then(|input| Some(rsx!(
                div {
                     h2 { "Keyboard Input" },
                     div {
                         box_sizing: "border-box",
                         background: "#DCDCDC",
                         height: "4rem",
                         width: "100%",
                         display: "flex",
                         align_items: "center",
                         border_radius: "4px",
                         padding: "1rem",
                         code {
                             [format_args!("input: {:?}, state: {:?}", input, state.unwrap())],
                         }
                     }
                }
            )))
        }
    ))
}
