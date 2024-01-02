mod ui;
mod util;
mod viewer;

use std::time::Duration;

use bevy::{
    prelude::*,
    window::PresentMode,
    winit::{UpdateMode, WinitSettings},
};

use ui::UIPlugin;
use viewer::ViewerPlugin;

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "KMPeek".into(),
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::ReactiveLowPower {
                wait: Duration::from_secs(60),
            },
            ..default()
        })
        .add_plugins((ViewerPlugin, UIPlugin))
        .run();
}
