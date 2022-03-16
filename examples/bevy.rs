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

#[derive(Debug, Clone)]
enum CoreCommand {
    Test,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyboardInput {
    pub scan_code: u32,
    pub key_code: Option<KeyCode>,
    pub state: ElementState,
}

#[derive(Debug, Clone, Copy)]
enum UICommand {
    Test,
    KeyboardInput(KeyboardInput),
}

fn main() {
    let mut config = DesktopConfig::default().with_default_icon();
    config.with_window(|w| w.with_title("Bevy Dioxus Plugin Demo"));

    App::new()
        .add_plugin(DioxusDesktopPlugin::<CoreCommand, UICommand>::new(app, ()))
        .insert_non_send_resource(config)
        .add_plugin(LogPlugin)
        .add_startup_system(setup)
        .add_system(handle_core_command)
        .add_system(send_keyboard_input)
        .run();
}

fn setup(mut event: EventWriter<UICommand>) {
    event.send(UICommand::Test);
}

fn handle_core_command(mut events: EventReader<CoreCommand>, mut event: EventWriter<AppExit>) {
    for cmd in events.iter() {
        info!("🧠 {:?}", cmd);

        match cmd {
            CoreCommand::Quit => event.send(AppExit),
            _ => {}
        }
    }
}

fn send_keyboard_input(
    mut events: EventReader<BevyKeyboardInput>,
    mut event: EventWriter<UICommand>,
) {
    for input in events.iter() {
        info!("🧠 {:?}", input);

        // copy KeyboardInput and send to UI
        event.send(UICommand::KeyboardInput(KeyboardInput {
            scan_code: input.scan_code,
            key_code: input.key_code,
            state: input.state,
        }));
    }
}

fn app(cx: Scope) -> Element {
    let window = use_bevy_window::<CoreCommand, UICommand>(&cx);
    let keyboard_input = use_state(&cx, || None);

    use_future(&cx, (), |_| {
        let keyboard_input = keyboard_input.clone();
        let mut rx = window.receiver();

        async move {
            while let Ok(cmd) = rx.recv().await {
                info!("🎨 {:?}", cmd);
                match cmd {
                    UICommand::KeyboardInput(input) => {
                        *keyboard_input.make_mut() = Some(input);
                    }
                    _ => {}
                }
            }
        }
    });

    cx.render(rsx! {
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
            keyboard_input.and_then(|input| {
                let input = format!("{:#?}", input);
                Some(rsx! {
                    div {
                        h2 { "Keyboard Input" },
                        code {
                            "{input}"
                        }
                    }
                })
            })
        }
    })
}
