use bevy::{
    app::{App, AppExit},
    ecs::event::{EventReader, EventWriter},
    log::{info, LogPlugin},
};
use dioxus::desktop::DioxusDesktopPlugin;
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
    App::new()
        .add_plugin(DioxusDesktopPlugin::<CoreCommand, UICommand>::new(app, ()))
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
    let ctx = dioxus::desktop::use_bevy_context::<CoreCommand, UICommand>(&cx);

    use_future(&cx, (), |_| {
        let mut rx = ctx.receiver();
        async move {
            while let Ok(cmd) = rx.recv().await {
                info!("🎨 {:?}", cmd);
            }
        }
    });

    cx.render(rsx! {
        div {
            h1 { "Bevy Plugin Example" },
            button {
                onclick: |_e| {
                    let _res = ctx.send(CoreCommand::Click);
                },
                "Click",
            }
            button {
                onclick: |_e| {
                    let _res = ctx.send(CoreCommand::Exit);
                },
                "Exit",
            }
        }
    })
}
