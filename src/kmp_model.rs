use std::{ffi::OsStr, fs::File};

use crate::{
    kmp_file::*,
    ui::{KclFileSelected, KmpFileSelected},
};
use bevy::{math::vec3, prelude::*};
use bevy_more_shapes::Cone;

pub struct KmpPlugin;

impl Plugin for KmpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_model, normalize_scale));
    }
}

#[derive(Component)]
pub struct KmpModelSection;

#[allow(clippy::comparison_chain)]
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

        commands.insert_resource(kmp.clone());

        let mut path = ev.0.clone();
        path.pop();
        path.push("course.kcl");
        if File::open(path.clone()).is_ok() {
            ev_kcl_file_selected.send(KclFileSelected(path));
        }

        // meshes for the kmp model
        let sphere = meshes.add(
            shape::UVSphere {
                radius: 100.,
                ..default()
            }
            .into(),
        );
        let cylinder_mesh = meshes.add(
            shape::Cylinder {
                radius: 100.,
                height: 1.,
                ..default()
            }
            .into(),
        );
        let cone_mesh = meshes.add(Mesh::from(Cone {
            radius: 100.,
            height: 200.,
            segments: 32,
        }));

        // materials
        let sphere_material = materials.add(StandardMaterial {
            base_color: Color::RED,
            unlit: true,
            ..default()
        });
        let group_line_material = materials.add(StandardMaterial {
            base_color: Color::ORANGE,
            unlit: true,
            ..default()
        });
        let join_line_material = materials.add(StandardMaterial {
            base_color: Color::GREEN,
            unlit: true,
            ..default()
        });
        let cone_material = materials.add(StandardMaterial {
            base_color: Color::YELLOW,
            unlit: true,
            ..default()
        });

        for group in kmp.itph.entries.iter() {
            // this contains the points of the current group
            let mut points = Vec::new();
            for i in group.start..(group.start + group.group_length) {
                points.push(kmp.itpt.entries[i as usize].clone());
            }
            for (i, point) in points.iter().enumerate() {
                // spawn the spheres where each point is
                commands.spawn((
                    PbrBundle {
                        mesh: sphere.clone(),
                        material: sphere_material.clone(),
                        transform: Transform::from_translation(point.position),
                        ..default()
                    },
                    NormalizeScale::new(200., 12., Vec3::ONE),
                    KmpModelSection,
                ));
                // if we are not at the end of the group
                if i < points.len() - 1 {
                    spawn_arrow_line(
                        &mut commands,
                        cylinder_mesh.clone(),
                        group_line_material.clone(),
                        cone_mesh.clone(),
                        cone_material.clone(),
                        point.position,
                        points[i + 1].position,
                    );
                } else if i == points.len() - 1 {
                    // draw a join line
                    for next_group_index in group.next_group {
                        if next_group_index as usize > kmp.itph.entries.len() {
                            continue;
                        }
                        let start_index = kmp.itph.entries[next_group_index as usize].start;

                        spawn_arrow_line(
                            &mut commands,
                            cylinder_mesh.clone(),
                            join_line_material.clone(),
                            cone_mesh.clone(),
                            cone_material.clone(),
                            point.position,
                            kmp.itpt.entries[start_index as usize].position,
                        );
                    }
                }
            }
        }
        commands.insert_resource(kmp);
    }
}

fn spawn_arrow_line(
    commands: &mut Commands,
    cylinder_mesh: Handle<Mesh>,
    cylinder_material: Handle<StandardMaterial>,
    cone_mesh: Handle<Mesh>,
    cone_material: Handle<StandardMaterial>,

    p1: Vec3,
    p2: Vec3,
) {
    let len = p1.distance(p2);
    let mut line_transform = Transform::from_translation((p1 + p2) / 2.).looking_at(p2, Vec3::Y);
    line_transform.scale.y = len;
    line_transform.rotate_local_x(f32::to_radians(90.));
    // spawn the line (cylinder)
    commands.spawn((
        PbrBundle {
            mesh: cylinder_mesh,
            material: cylinder_material,
            transform: line_transform,
            ..default()
        },
        NormalizeScale::new(200., 8., Vec3::X + Vec3::Z),
        KmpModelSection,
    ));

    let mut arrowhead_transform =
        Transform::from_translation(p1.lerp(p2, 0.5)).looking_at(p2, Vec3::Y);
    arrowhead_transform.rotate_local_x(f32::to_radians(-90.));
    commands.spawn((
        PbrBundle {
            mesh: cone_mesh,
            material: cone_material,
            transform: arrowhead_transform,
            ..default()
        },
        NormalizeScale::new(200., 20., Vec3::ONE),
        KmpModelSection,
    ));
}

/// Marker struct that marks entities with meshes that should be scaled relative to the camera.
#[derive(Component, Debug)]
pub struct NormalizeScale {
    /// Length of the object in world space units
    pub size_in_world: f32,
    /// Desired length of the object in pixels
    pub desired_pixel_size: f32,
    pub axes: Vec3,
    pub multiplier: f32,
}
impl NormalizeScale {
    pub fn new(size_in_world: f32, desired_pixel_size: f32, axes: Vec3) -> Self {
        Self {
            size_in_world,
            desired_pixel_size,
            axes,
            multiplier: 1.,
        }
    }
}
#[allow(clippy::type_complexity)]
pub fn normalize_scale(
    mut query: ParamSet<(
        Query<(&GlobalTransform, &Camera)>,
        Query<(&mut Transform, &mut GlobalTransform, &NormalizeScale)>,
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

        let Some(pixel_end) = camera.world_to_viewport(
            &GlobalTransform::default(),
            Vec3::new(normalize.size_in_world * gt.scale.x, 0.0, distance)
        ) else {continue};

        let Some(pixel_root) = camera.world_to_viewport(
            &GlobalTransform::default(),
            Vec3::new(0.0, 0.0, distance)
        ) else {continue};

        let actual_pixel_size = pixel_root.distance(pixel_end);

        let required_scale =
            (normalize.desired_pixel_size * normalize.multiplier) / actual_pixel_size;

        let scale_before = gt.scale;

        transform.scale = gt.scale * Vec3::splat(required_scale);

        if normalize.axes.x == 0. {
            transform.scale.x = scale_before.x;
        }
        if normalize.axes.y == 0. {
            transform.scale.y = scale_before.y;
        }
        if normalize.axes.z == 0. {
            transform.scale.z = scale_before.z;
        }

        *global_transform = (*transform).into();
    }
}
