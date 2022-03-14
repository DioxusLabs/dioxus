use bevy::{
    app::{App, AppExit},
    ecs::event::{EventReader, EventWriter},
    input::keyboard::KeyboardInput,
    log::{info, LogPlugin},
};
use dioxus::prelude::*;

#[derive(Debug, Clone)]
enum CoreCommand {
    Test,
    Quit,
}

#[derive(Debug, Clone, Copy)]
enum UICommand {
    Test,
}

fn main() {
    let mut config = DesktopConfig::default().with_default_icon();
    config.with_window(|w| w.with_title("Bevy Dioxus Plugin Demo"));

    App::new()
        .add_plugin(DioxusDesktopPlugin::<CoreCommand, UICommand>::new(app, ()))
        .insert_non_send_resource(config)
        .add_plugin(LogPlugin)
        .add_startup_system(setup)
        .add_system(log_core_command)
        .add_system(log_keyboard_input)
        .run();
}

fn setup(mut event: EventWriter<UICommand>) {
    event.send(UICommand::Test);
}

fn log_core_command(mut events: EventReader<CoreCommand>, mut event: EventWriter<AppExit>) {
    for cmd in events.iter() {
        info!("🧠 {:?}", cmd);

        match cmd {
            CoreCommand::Quit => event.send(AppExit),
            _ => {}
        }
    }
}

fn log_keyboard_input(mut events: EventReader<KeyboardInput>) {
    for input in events.iter() {
        info!("🧠 {:?}", input);
    }
}

fn app(cx: Scope) -> Element {
    let window = use_bevy_window::<CoreCommand, UICommand>(&cx);

    use_bevy_listener::<CoreCommand, UICommand>(&cx, |cmd| {
        info!("🎨 {:?}", cmd);
    });

    cx.render(rsx! {
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
    })
}
