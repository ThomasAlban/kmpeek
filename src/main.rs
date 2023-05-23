mod kcl;
mod kmp;

use kcl::*;
use kmp::*;

use bevy::prelude::*;
use smooth_bevy_cameras::{
    controllers::fps::{FpsCameraBundle, FpsCameraController, FpsCameraPlugin},
    LookTransformPlugin,
};
use std::fs::File;

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins(DefaultPlugins)
        .add_plugin(LookTransformPlugin)
        .add_plugin(FpsCameraPlugin::default())
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let name = "dry_dry_ruins";

    let kcl_file = File::open(format!("{name}.kcl")).unwrap();
    let kcl = KCL::read(kcl_file).unwrap();

    let kmp_file = File::open(format!("{name}.kmp")).unwrap();
    let kmp = KMP::read(kmp_file).unwrap();

    kcl.build_model(&mut commands, &mut meshes, &mut materials);

    kmp.build_model(&mut commands, &mut meshes, &mut materials);

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.5,
    });

    commands
        .spawn(Camera3dBundle::default())
        .insert(FpsCameraBundle::new(
            FpsCameraController {
                smoothing_weight: 0.7,
                translate_sensitivity: 10000.,
                ..default()
            },
            Vec3::new(-2.0, 5.0, 5.0),
            Vec3::new(0., 0., 0.),
            Vec3::Y,
        ));
}
