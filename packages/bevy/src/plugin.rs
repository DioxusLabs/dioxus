use crate::{
    context::UserEvent,
    event::{DomUpdated, DragWindow, UpdateDom, UpdateVisible, VisibleUpdated, WindowDragged},
    runner::runner,
    setting::DioxusSettings,
    window::DioxusWindows,
};
use bevy::{
    app::prelude::*,
    ecs::{event::Events, prelude::*},
    input::InputPlugin,
    log::error,
    window::{
        CreateWindow, WindowCommand, WindowCreated, WindowMode, WindowPlugin,
        WindowScaleFactorChanged, Windows,
    },
};
use dioxus_core::Component as DioxusComponent;
use dioxus_desktop::{
    cfg::DesktopConfig,
    tao::{
        dpi::{LogicalPosition, LogicalSize, PhysicalPosition},
        event_loop::EventLoop,
        window::Fullscreen,
    },
};
use futures_intrusive::channel::shared::{channel, Sender};
use std::{fmt::Debug, marker::PhantomData};
use tokio::runtime::Runtime;

pub struct DioxusDesktopPlugin<CoreCommand, UICommand, Props = ()> {
    pub root: DioxusComponent<Props>,
    pub props: Props,
    core_cmd_type: PhantomData<CoreCommand>,
    ui_cmd_type: PhantomData<UICommand>,
}

impl<CoreCommand, UICommand, Props> Plugin for DioxusDesktopPlugin<CoreCommand, UICommand, Props>
where
    CoreCommand: 'static + Send + Sync + Clone + Debug,
    UICommand: 'static + Send + Sync + Clone + Debug,
    Props: 'static + Send + Sync + Copy,
{
    fn build(&self, app: &mut App) {
        let runtime = Runtime::new().unwrap();

        let (core_tx, core_rx) = channel::<CoreCommand>(8);
        let (ui_tx, ui_rx) = channel::<UICommand>(8);
        let config = app
            .world
            .remove_non_send_resource::<DesktopConfig>()
            .unwrap_or_default();
        let settings = app
            .world
            .remove_non_send_resource::<DioxusSettings>()
            .unwrap_or_default();

        let event_loop = EventLoop::<UserEvent<CoreCommand>>::with_user_event();

        app.add_plugin(InputPlugin)
            .add_plugin(WindowPlugin::default())
            .add_event::<CoreCommand>()
            .add_event::<UICommand>()
            .add_event::<UpdateDom>()
            .add_event::<DomUpdated>()
            .add_event::<DragWindow>()
            .add_event::<WindowDragged>()
            .add_event::<UpdateVisible>()
            .add_event::<VisibleUpdated>()
            .insert_resource(core_tx)
            .insert_resource(core_rx)
            .insert_resource(ui_tx)
            .insert_resource(ui_rx)
            .insert_resource(runtime)
            .insert_resource(self.root)
            .insert_resource(self.props)
            .insert_resource(settings)
            .insert_non_send_resource(config)
            .init_non_send_resource::<DioxusWindows>()
            .set_runner(|app| runner::<CoreCommand, UICommand, Props>(app))
            .insert_non_send_resource(event_loop)
            .add_system_to_stage(CoreStage::Last, send_ui_commands::<UICommand>)
            .add_system_to_stage(
                CoreStage::PostUpdate,
                change_window, /* TODO.label(ModifiesWindows) // is recentry introduced ( > 0.7 ) */
            )
            .add_system(handle_updated_dom)
            .add_system(handle_drag_window)
            .add_system(handle_update_visible);

        Self::handle_initial_window_events(&mut app.world);
    }
}

impl<CoreCommand, UICommand, Props> DioxusDesktopPlugin<CoreCommand, UICommand, Props> {
    fn handle_initial_window_events(world: &mut World)
    where
        CoreCommand: 'static + Send + Sync + Clone + Debug,
        UICommand: 'static + Send + Sync + Clone + Debug,
        Props: 'static + Send + Sync + Copy,
    {
        let world = world.cell();
        let mut dioxus_windows = world.get_non_send_mut::<DioxusWindows>().unwrap();
        let mut bevy_windows = world.get_resource_mut::<Windows>().unwrap();
        let mut create_window_events = world.get_resource_mut::<Events<CreateWindow>>().unwrap();
        let mut window_created_events = world.get_resource_mut::<Events<WindowCreated>>().unwrap();

        for create_window_event in create_window_events.drain() {
            let window = dioxus_windows.create::<CoreCommand, UICommand, Props>(
                &world,
                create_window_event.id,
                &create_window_event.descriptor,
            );
            bevy_windows.add(window);
            window_created_events.send(WindowCreated {
                id: create_window_event.id,
            });
        }
    }
}

impl<CoreCommand, UICommand, Props> DioxusDesktopPlugin<CoreCommand, UICommand, Props> {
    pub fn new(root: DioxusComponent<Props>, props: Props) -> Self {
        Self {
            root,
            props,
            core_cmd_type: PhantomData,
            ui_cmd_type: PhantomData,
        }
    }
}

fn send_ui_commands<UICommand>(mut events: EventReader<UICommand>, tx: Res<Sender<UICommand>>)
where
    UICommand: 'static + Send + Sync + Clone,
{
    for e in events.iter() {
        if let Err(_) = tx.try_send(e.clone()) {
            error!("Failed to send UICommand");
        };
    }
}

fn change_window(
    mut dioxus_windows: NonSendMut<DioxusWindows>,
    mut windows: ResMut<Windows>,
    mut window_dpi_changed_events: EventWriter<WindowScaleFactorChanged>,
    // mut window_close_events: EventWriter<WindowClosed>,
) {
    // let mut removed_windows = vec![];

    for bevy_window in windows.iter_mut() {
        let id = bevy_window.id();
        for command in bevy_window.drain_commands() {
            match command {
                WindowCommand::SetWindowMode {
                    mode,
                    resolution: (width, height),
                } => {
                    let window = dioxus_windows.get_tao_window(id).unwrap();
                    match mode {
                        WindowMode::BorderlessFullscreen => {
                            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                        }
                        WindowMode::Fullscreen => {
                            window.set_fullscreen(Some(Fullscreen::Exclusive(
                                DioxusWindows::get_best_videomode(
                                    &window.current_monitor().unwrap(),
                                ),
                            )));
                        }
                        WindowMode::SizedFullscreen => window.set_fullscreen(Some(
                            Fullscreen::Exclusive(DioxusWindows::get_fitting_videomode(
                                &window.current_monitor().unwrap(),
                                width,
                                height,
                            )),
                        )),
                        WindowMode::Windowed => window.set_fullscreen(None),
                    }
                }
                WindowCommand::SetTitle { title } => {
                    let window = dioxus_windows.get_tao_window(id).unwrap();
                    window.set_title(&title);
                }
                WindowCommand::SetScaleFactor { scale_factor } => {
                    window_dpi_changed_events.send(WindowScaleFactorChanged { id, scale_factor });
                }
                WindowCommand::SetResolution {
                    logical_resolution: (width, height),
                    scale_factor,
                } => {
                    let window = dioxus_windows.get_tao_window(id).unwrap();
                    window.set_inner_size(
                        LogicalSize::new(width, height).to_physical::<f64>(scale_factor),
                    );
                }
                WindowCommand::SetPresentMode { .. } => (),
                WindowCommand::SetResizable { resizable } => {
                    let window = dioxus_windows.get_tao_window(id).unwrap();
                    window.set_resizable(resizable);
                }
                WindowCommand::SetDecorations { decorations } => {
                    let window = dioxus_windows.get_tao_window(id).unwrap();
                    window.set_decorations(decorations);
                }
                WindowCommand::SetCursorIcon { icon } => {
                    // let window = dioxus_windows.get_tao_window(id).unwrap();
                    // window.set_cursor_icon(converters::convert_cursor_icon(icon));
                }
                WindowCommand::SetCursorLockMode { locked } => {
                    let window = dioxus_windows.get_tao_window(id).unwrap();
                    window
                        .set_cursor_grab(locked)
                        .unwrap_or_else(|e| error!("Unable to un/grab cursor: {}", e));
                }
                WindowCommand::SetCursorVisibility { visible } => {
                    let window = dioxus_windows.get_tao_window(id).unwrap();
                    window.set_cursor_visible(visible);
                }
                WindowCommand::SetCursorPosition { position } => {
                    let window = dioxus_windows.get_tao_window(id).unwrap();
                    let inner_size = window.inner_size().to_logical::<f32>(window.scale_factor());
                    window
                        .set_cursor_position(LogicalPosition::new(
                            position.x,
                            inner_size.height - position.y,
                        ))
                        .unwrap_or_else(|e| error!("Unable to set cursor position: {}", e));
                }
                WindowCommand::SetMaximized { maximized } => {
                    let window = dioxus_windows.get_tao_window(id).unwrap();
                    window.set_maximized(maximized);
                }
                WindowCommand::SetMinimized { minimized } => {
                    let window = dioxus_windows.get_tao_window(id).unwrap();
                    window.set_minimized(minimized);
                }
                WindowCommand::SetPosition { position } => {
                    let window = dioxus_windows.get_tao_window(id).unwrap();
                    window.set_outer_position(PhysicalPosition {
                        x: position[0],
                        y: position[1],
                    });
                }
                WindowCommand::SetResizeConstraints { resize_constraints } => {
                    let window = dioxus_windows.get_tao_window(id).unwrap();
                    let constraints = resize_constraints.check_constraints();
                    let min_inner_size = LogicalSize {
                        width: constraints.min_width,
                        height: constraints.min_height,
                    };
                    let max_inner_size = LogicalSize {
                        width: constraints.max_width,
                        height: constraints.max_height,
                    };

                    window.set_min_inner_size(Some(min_inner_size));
                    if constraints.max_width.is_finite() && constraints.max_height.is_finite() {
                        window.set_max_inner_size(Some(max_inner_size));
                    }
                } // WindowCommand::Close => {
                  //     // Since we have borrowed `windows` to iterate through them, we can't remove the window from it.
                  //     // Add the removal requests to a queue to solve this
                  //     removed_windows.push(id);
                  //     // No need to run any further commands - this drops the rest of the commands, although the `bevy_window::Window` will be dropped later anyway
                  //     break;
                  // }
            }
        }
    }

    // if !removed_windows.is_empty() {
    //     for id in removed_windows {
    //         // Close the OS window. (The `Drop` impl actually closes the window)
    //         let _ = dioxus_windows.remove_window(id);
    //         // Clean up our own data structures
    //         windows.remove(id);
    //         window_close_events.send(WindowClosed { id });
    //     }
    // }
}

fn handle_updated_dom(
    mut events: EventReader<UpdateDom>,
    mut event: EventWriter<DomUpdated>,
    mut windows: NonSendMut<DioxusWindows>,
) {
    for UpdateDom { id } in events.iter() {
        let window = windows.get_mut(*id).unwrap();
        window.try_load_ready_webview();

        event.send(DomUpdated { id: *id });
    }
}

fn handle_drag_window(
    mut events: EventReader<DragWindow>,
    mut event: EventWriter<WindowDragged>,
    mut windows: NonSendMut<DioxusWindows>,
) {
    for e in events.iter() {
        let window = windows.get(e.id).unwrap();
        let tao_window = window.tao_window();

        if tao_window.fullscreen().is_none() {
            if let Ok(()) = tao_window.drag_window() {
                event.send(WindowDragged { id: e.id });
            }
        }
    }
}

fn handle_update_visible(
    mut events: EventReader<UpdateVisible>,
    mut event: EventWriter<VisibleUpdated>,
    mut windows: NonSendMut<DioxusWindows>,
) {
    for e in events.iter() {
        let window = windows.get(e.id).unwrap();
        let tao_window = window.tao_window();

        tao_window.set_visible(e.visible);
        event.send(VisibleUpdated {
            id: e.id,
            visible: e.visible,
        });
    }
}
