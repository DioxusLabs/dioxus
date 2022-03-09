use bevy::{
    app::{App, AppExit},
    ecs::event::{EventReader, EventWriter},
    log::{info, LogPlugin},
};
use dioxus::{
    desktop::{DesktopConfig, DioxusDesktopPlugin},
    prelude::*,
};

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
            h1 { "Bevy Dioxus Plugin Example" },
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
