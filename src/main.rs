mod camera;
mod kcl;
mod kmp;
mod ui;

use camera::*;
use kcl::*;
use kmp::*;
use ui::*;

use bevy::{
    prelude::*,
    winit::{UpdateMode, WinitSettings},
};

use std::{fs::File, time::Duration};

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
                max_wait: Duration::MAX,
            },
            ..default()
        })
        .add_plugin(UIPlugin)
        .add_plugin(CameraPlugin)
        .add_plugin(KCLPlugin)
        .add_plugin(KMPPlugin)
        // make sure this startup system runs before spawning the models
        .add_startup_system(setup.in_base_set(StartupSet::PreStartup))
        .run();
}

fn setup(mut commands: Commands) {
    let name = "dry_dry_ruins";

    let kcl_file = File::open(format!("{name}.kcl")).unwrap();
    commands.insert_resource(Kcl::read(kcl_file).unwrap());

    let kmp_file = File::open(format!("{name}.kmp")).unwrap();
    commands.insert_resource(Kmp::read(kmp_file).unwrap());

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.5,
    });
}
