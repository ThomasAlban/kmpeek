mod camera;
mod kcl_file;
mod kcl_model;
mod kmp_file;
mod kmp_model;
mod mouse_picking;
mod ui;
mod undo;

use camera::*;
use kcl_model::*;
use kmp_model::*;
use mouse_picking::*;
use ui::*;
use undo::*;

use bevy::{
    prelude::*,
    winit::{UpdateMode, WinitSettings},
};

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "KMPeek".into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::ReactiveLowPower {
                max_wait: std::time::Duration::MAX,
            },
            ..default()
        })
        .add_plugins((
            CameraPlugin,
            UIPlugin,
            KclPlugin,
            KmpPlugin,
            MousePickingPlugin,
            UndoPlugin,
        ))
        .run();
}
