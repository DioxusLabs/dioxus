use bevy::prelude::*;
use dioxus::desktop::{AppProps, DioxusDesktop, DioxusDesktopPlugin};
use dioxus::prelude::*;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
enum CoreCommand {
    Click,
}

#[derive(Debug, Clone, Copy)]
enum UICommand {
    Test,
}

fn main() {
    App::new()
        .add_plugin(DioxusDesktopPlugin::<CoreCommand, UICommand> {
            root: app,
            core_cmd_type: PhantomData,
            ui_cmd_type: PhantomData,
        })
        .add_startup_system(setup)
        .add_system(notify_core_command)
        .run();
}

fn setup(desktop: Res<DioxusDesktop<CoreCommand, UICommand>>) {
    println!("setup");
    let _res = desktop.sender().send(UICommand::Test);
}

fn notify_core_command(mut events: EventReader<CoreCommand>) {
    for e in events.iter() {
        println!("🧠 {:?}", e);
    }
}

fn app(cx: Scope<AppProps<CoreCommand, UICommand>>) -> Element {
    use_future(&cx, (), |_| {
        let mut rx = cx.props.channel.1.subscribe();
        async move {
            while let Ok(cmd) = rx.recv().await {
                println!("🎨 {:?}", cmd);
            }
        }
    });

    cx.render(rsx! {
        div {
            h1 { "Bevy Plugin Example" },
            button {
                onclick: |_e| {
                    let _res = cx.props.channel.0.send(CoreCommand::Click);
                },
                "Send",
            }
        }
    })
}
