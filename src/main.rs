mod camera_input;
mod kcl;
mod kmp;

use camera_input::*;
use kcl::*;
use kmp::*;

use bevy::{
    prelude::*,
    render::{
        settings::{Backends, WgpuSettings},
        RenderPlugin,
    },
};
use bevy_mod_picking::{prelude::RaycastPickCamera, DefaultPickingPlugins};
use smooth_bevy_cameras::{
    controllers::fps::{FpsCameraBundle, FpsCameraController, FpsCameraPlugin},
    LookTransformPlugin,
};
use std::fs::File;

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "KMPeek".into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    wgpu_settings: WgpuSettings {
                        backends: Some(Backends::DX12),
                        ..default()
                    },
                }),
        )
        .add_plugin(LookTransformPlugin)
        .add_plugin(FpsCameraPlugin {
            override_input_system: true,
        })
        .add_plugins(DefaultPickingPlugins)
        .add_system(camera_input)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let name = "maple_treeway";

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
        .spawn((Camera3dBundle::default(), RaycastPickCamera::default()))
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
