mod camera;
mod kcl_file;
mod kcl_model;
mod kmp_file;
mod kmp_model;
mod ui;

use std::fs::File;

use camera::*;
use kcl_file::Kcl;
use kcl_model::*;
use ui::*;

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
        .insert_resource(Kcl::read(File::open("dry_dry_ruins.kcl").unwrap()).unwrap())
        .add_plugin(CameraPlugin)
        .add_plugin(UIPlugin)
        .add_plugin(KclPlugin)
        // .add_plugin(KmpPlugin)
        .run();
}
