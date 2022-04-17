use bevy::{
    app::{App, AppExit},
    ecs::event::{EventReader, EventWriter},
    input::{
        keyboard::{KeyCode, KeyboardInput as BevyKeyboardInput},
        ElementState,
    },
    log::{info, LogPlugin},
};
use dioxus::prelude::*;
use std::convert::From;

#[derive(Debug, Clone)]
enum CoreCommand {
    Test,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
// mimic bevy's KeyboardInput since it doesn't implement Copy.
pub struct KeyboardInput {
    pub scan_code: u32,
    pub key_code: Option<KeyCode>,
    pub state: ElementState,
}

impl From<&BevyKeyboardInput> for KeyboardInput {
    fn from(input: &BevyKeyboardInput) -> Self {
        KeyboardInput {
            scan_code: input.scan_code,
            key_code: input.key_code,
            state: input.state,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum UICommand {
    Test,
    KeyboardInput(KeyboardInput),
}

impl From<&BevyKeyboardInput> for UICommand {
    fn from(input: &BevyKeyboardInput) -> Self {
        UICommand::KeyboardInput(input.into())
    }
}

fn main() {
    let mut config = DesktopConfig::default().with_default_icon();
    config.with_window(|w| w.with_title("Bevy Dioxus Plugin Demo"));

    App::new()
        .add_plugin(DioxusDesktopPlugin::<CoreCommand, UICommand>::new(app, ()))
        .insert_non_send_resource(config)
        .add_plugin(LogPlugin)
        .add_system(handle_core_command)
        .add_system(send_keyboard_input)
        .run();
}

fn handle_core_command(
    mut events: EventReader<CoreCommand>,
    mut exit: EventWriter<AppExit>,
    mut ui: EventWriter<UICommand>,
) {
    for cmd in events.iter() {
        info!("🧠 {:?}", cmd);

        match cmd {
            CoreCommand::Quit => exit.send(AppExit),
            CoreCommand::Test => ui.send(UICommand::Test),
        }
    }
}

fn send_keyboard_input(
    mut events: EventReader<BevyKeyboardInput>,
    mut event: EventWriter<UICommand>,
) {
    for input in events.iter() {
        info!("🧠 {:?}", input);

        event.send(input.into());
    }
}

fn app(cx: Scope) -> Element {
    let window = use_bevy_window::<CoreCommand, UICommand>(&cx);
    let ui_cmd = use_state(&cx, || None);
    let keyboard_input = use_state(&cx, || None);

    use_future(&cx, (), |_| {
        let keyboard_input = keyboard_input.clone();
        let ui_cmd = ui_cmd.clone();
        let mut rx = window.receiver();

        async move {
            while let Ok(cmd) = rx.recv().await {
                *ui_cmd.make_mut() = Some(cmd);

                match cmd {
                    UICommand::KeyboardInput(input) => {
                        *keyboard_input.make_mut() = Some(input);
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
                    onclick: |_e| {
                        window.send(CoreCommand::Test).unwrap();
                    },
                    "Test",
                }
                button {
                    onclick: |_e| {
                        window.send(CoreCommand::Quit).unwrap();
                    },
                    "Quit",
                }
                button {
                    onclick: move |_| window.set_minimized(true),
                    "Minimize"
                }
            }

            ui_cmd.and_then(|cmd| Some(rsx!(
                div {
                     h2 { "UI Command" },
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
                             [format_args!("{:#?}", cmd)],
                         }
                     }
                }
            )))
        }
    ))
}
