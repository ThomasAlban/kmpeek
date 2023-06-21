use std::{ffi::OsStr, fs::File};

use crate::{
    kmp_file::*,
    ui::{KclFileSelected, KmpFileSelected},
};
use bevy::prelude::*;

pub struct KmpPlugin;

impl Plugin for KmpPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_model).add_system(normalize);
    }
}

#[derive(Component)]
pub struct KmpModelSection;

pub fn spawn_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ev_kmp_file_selected: EventReader<KmpFileSelected>,
    mut ev_kcl_file_selected: EventWriter<KclFileSelected>,
    mut model: Query<Entity, With<KmpModelSection>>,
) {
    for ev in ev_kmp_file_selected.iter() {
        if ev.0.extension() != Some(OsStr::new("kmp")) {
            continue;
        }
        // despawn all entities with KmpModelSection (so that we have a clean slate)
        for entity in model.iter_mut() {
            commands.entity(entity).despawn();
        }
        commands.remove_resource::<Kmp>();

        // open the KMP file and read it
        let kmp_file = File::open(ev.0.clone()).unwrap();
        let kmp = Kmp::read(kmp_file).unwrap();

        let mut path = ev.0.clone();
        path.pop();
        path.push("course.kcl");
        if File::open(path.clone()).is_ok() {
            ev_kcl_file_selected.send(KclFileSelected(path));
        }

        let sphere = meshes.add(
            shape::UVSphere {
                radius: 200.,
                ..default()
            }
            .into(),
        );

        let sphere_material = materials.add(Color::RED.into());
        let cylinder_material = materials.add(Color::ORANGE.into());

        for group in kmp.itph.entries.iter() {
            let mut points = Vec::new();
            for i in group.start..(group.start + group.group_length) {
                points.push(kmp.itpt.entries[i as usize]);
            }
            for (i, point) in points.iter().enumerate() {
                commands.spawn((
                    PbrBundle {
                        mesh: sphere.clone(),
                        material: sphere_material.clone(),
                        transform: Transform::from_translation(point.position),
                        ..default()
                    },
                    // Normalize3d::new(2., 12.),
                    KmpModelSection,
                ));

                if i < points.len() - 1 {
                    let p1 = point.position;
                    let p2 = points[i + 1].position;

                    let len = p1.distance(p2);
                    let mut transform =
                        Transform::from_translation((p1 + p2) / 2.).looking_at(p2, Vec3::Y);
                    transform.rotate_local_x(f32::to_radians(90.));

                    let cylinder = meshes.add(
                        shape::Cylinder {
                            radius: 150.,
                            height: len,
                            ..default()
                        }
                        .into(),
                    );

                    commands.spawn((
                        PbrBundle {
                            mesh: cylinder.clone(),
                            material: cylinder_material.clone(),
                            transform,
                            ..default()
                        },
                        KmpModelSection,
                    ));
                }
            }
        }
        commands.insert_resource(kmp);
    }
}

/// Marker struct that marks entities with meshes that should be scaled relative to the camera.
#[derive(Component, Debug)]
pub struct Normalize3d {
    /// Length of the object in world space units
    pub size_in_world: f32,
    /// Desired length of the object in pixels
    pub desired_pixel_size: f32,
}
impl Normalize3d {
    pub fn new(size_in_world: f32, desired_pixel_size: f32) -> Self {
        Normalize3d {
            size_in_world,
            desired_pixel_size,
        }
    }
}
#[allow(clippy::type_complexity)]
pub fn normalize(
    mut query: ParamSet<(
        Query<(&GlobalTransform, &Camera)>,
        Query<(&mut Transform, &mut GlobalTransform, &Normalize3d)>,
    )>,
) {
    let (mut camera_position, mut camera) = (None, None);
    for cam in query.p0().iter() {
        if cam.1.is_active {
            if camera.is_some() {
                panic!("More than one active camera");
            }
            (camera_position, camera) = (Some(cam.0.to_owned()), Some(cam.1.to_owned()));
        }
    }
    let camera_position = camera_position.expect("Could not find active camera");
    let camera = camera.expect("Could not find active camera");

    let view = camera_position.compute_matrix().inverse();

    for (mut transform, mut global_transform, normalize) in query.p1().iter_mut() {
        let distance = view.transform_point3(global_transform.translation()).z;
        let gt = global_transform.compute_transform();
        let pixel_end = if let Some(coords) = Camera::world_to_viewport(
            &camera,
            &GlobalTransform::default(),
            Vec3::new(normalize.size_in_world * gt.scale.x, 0.0, distance),
        ) {
            coords
        } else {
            continue;
        };
        let pixel_root = if let Some(coords) = Camera::world_to_viewport(
            &camera,
            &GlobalTransform::default(),
            Vec3::new(0.0, 0.0, distance),
        ) {
            coords
        } else {
            continue;
        };
        let actual_pixel_size = pixel_root.distance(pixel_end);
        let required_scale = normalize.desired_pixel_size / actual_pixel_size;
        transform.scale = gt.scale * Vec3::splat(required_scale);
        *global_transform = (*transform).into();
    }
}
