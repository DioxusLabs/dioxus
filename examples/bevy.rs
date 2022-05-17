use bevy::{
    app::App,
    core::CorePlugin,
    ecs::{
        component::Component,
        event::{EventReader, EventWriter},
        query::With,
        system::{Commands, Query},
    },
    input::keyboard::{KeyCode, KeyboardInput},
    log::{info, LogPlugin},
    window::{ReceivedCharacter, WindowCloseRequested, WindowDescriptor, WindowId},
};
use dioxus::prelude::*;
use leafwing_input_manager::prelude::*;

#[derive(Debug, Clone)]
enum CoreCommand {
    Test,
}

#[derive(Debug, Clone)]
enum UICommand {
    Test,
    KeyboardInput(KeyboardInput),
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum Action {
    CloseWindow,
}

#[derive(Component)]
struct User;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Bevy Dioxus Plugin Demo".to_string(),
            ..Default::default()
        })
        .add_plugin(DioxusDesktopPlugin::<CoreCommand, UICommand>::new(app, ()))
        .add_plugin(LogPlugin)
        .add_system(send_keyboard_input)
        .add_system(handle_core_command)
        .add_plugin(CorePlugin)
        .add_plugin(InputManagerPlugin::<Action>::default())
        .add_system(log_keyboard_event)
        .add_startup_system(spawn_user)
        .add_system(close_window)
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
                h1 { "Bevy Dioxus Plugin Example" }
                div {
                    p {"Test CoreCommand chanel"}
                    button {
                        onclick: |_e| window.send(CoreCommand::Test),
                        "Test",
                    }
                }
                div {
                    p {"Close Window (press Esc or Ctrl + c)"}
                    button {
                        onclick: |_e| window.close(),
                        "Close",
                    }
                }
                div {
                    p { "Minimize Window" }
                    button {
                        onclick: move |_| window.set_minimized(true),
                        "Minimize"
                    }
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

fn spawn_user(mut commands: Commands) {
    let mut input_map = InputMap::new([(Action::CloseWindow, KeyCode::Escape)]);
    input_map.insert_chord(Action::CloseWindow, [KeyCode::LControl, KeyCode::C]);
    input_map.insert_chord(Action::CloseWindow, [KeyCode::RControl, KeyCode::C]);

    commands
        .spawn()
        .insert(User)
        .insert_bundle(InputManagerBundle::<Action> {
            action_state: ActionState::default(),
            input_map,
        });
}

fn close_window(
    query: Query<&ActionState<Action>, With<User>>,
    mut events: EventWriter<WindowCloseRequested>,
) {
    let action_state = query.single();
    if action_state.just_pressed(Action::CloseWindow) {
        events.send(WindowCloseRequested {
            id: WindowId::primary(),
        });
    }
}
