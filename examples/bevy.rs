use bevy_app::{App, AppExit};
use bevy_dioxus::{use_bevy_context, use_bevy_listener, DesktopConfig, DioxusDesktopPlugin};
use bevy_ecs::event::{EventReader, EventWriter};
use bevy_log::{info, LogPlugin};
use dioxus::prelude::*;

#[derive(Debug, Clone)]
enum CoreCommand {
    Click,
    Exit,
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
        .run();
}

fn setup(mut event: EventWriter<UICommand>) {
    event.send(UICommand::Test);
}

fn log_core_command(mut events: EventReader<CoreCommand>, mut event: EventWriter<AppExit>) {
    for cmd in events.iter() {
        info!("🧠 {:?}", cmd);

        match cmd {
            CoreCommand::Exit => event.send(AppExit),
            _ => {}
        }
    }
}

fn app(cx: Scope) -> Element {
    let ctx = use_bevy_context::<CoreCommand, UICommand>(&cx);

    use_bevy_listener::<CoreCommand, UICommand>(&cx, |cmd| {
        info!("🎨 {:?}", cmd);
    });

    cx.render(rsx! {
        div {
            h1 { "Bevy Dioxus Plugin Example" },
            button {
                onclick: |_e| {
                    ctx.send(CoreCommand::Click).unwrap();
                },
                "Click",
            }
            button {
                onclick: |_e| {
                    ctx.send(CoreCommand::Exit).unwrap();
                },
                "Exit",
            }
        }
    })
}
