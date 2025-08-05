mod bevy_scene_plugin;
mod dioxus_in_bevy_plugin;
mod ui;

use crate::bevy_scene_plugin::BevyScenePlugin;
use crate::dioxus_in_bevy_plugin::DioxusInBevyPlugin;
use crate::ui::{ui, UIProps};
use bevy::prelude::*;

fn main() {
    #[cfg(feature = "tracing")]
    tracing_subscriber::fmt::init();

    let (ui_sender, ui_receiver) = crossbeam_channel::unbounded();
    let (app_sender, app_receiver) = crossbeam_channel::unbounded();
    let props = UIProps {
        ui_sender,
        app_receiver,
    };

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DioxusInBevyPlugin::<UIProps> { ui, props })
        .add_plugins(BevyScenePlugin {
            app_sender,
            ui_receiver,
        })
        .run();
}
