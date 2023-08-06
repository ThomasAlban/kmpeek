use std::{ffi::OsStr, fs::File};

use crate::{
    kmp_file::*,
    ui::{KclFileSelected, KmpFileSelected},
};
use bevy::prelude::*;
use bevy_more_shapes::Cylinder;

pub struct KmpPlugin;

impl Plugin for KmpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_model);
        // normalize has to run after update_itpt otherwise the transform will be overwritten
        app.add_systems(Update, (update_itpt, normalize_scale).chain());
    }
}

#[derive(Component)]
pub struct KmpModelSection;

#[derive(Component, Deref)]
pub struct ItptModel(pub usize);

#[derive(Component)]
pub struct ItptArrowLine {
    p1: usize,
    p2: usize,
    is_line: bool,
}

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
        let sphere_mesh = meshes.add(
            shape::UVSphere {
                radius: 100.,
                ..default()
            }
            .into(),
        );
        let cylinder_mesh = meshes.add(Mesh::from(Cylinder {
            height: 1.,
            radius_bottom: 100.,
            radius_top: 100.,
            radial_segments: 32,
            height_segments: 32,
        }));
        let cone_mesh = meshes.add(Mesh::from(Cylinder {
            height: 100.,
            radius_bottom: 100.,
            radius_top: 50.,
            radial_segments: 32,
            height_segments: 32,
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
                points.push((kmp.itpt.entries[i as usize].clone(), i as usize));
            }
            for (i, point) in points.iter().enumerate() {
                // spawn the spheres where each point is
                commands.spawn((
                    PbrBundle {
                        mesh: sphere_mesh.clone(),
                        material: sphere_material.clone(),
                        transform: Transform::from_translation(point.0.position),
                        ..default()
                    },
                    NormalizeScale::new(200., 12., Vec3::ONE),
                    KmpModelSection,
                    ItptModel(point.1),
                ));
                // if we are not at the end of the group
                if i < points.len() - 1 {
                    spawn_arrow_line(
                        &mut commands,
                        cylinder_mesh.clone(),
                        group_line_material.clone(),
                        cone_mesh.clone(),
                        cone_material.clone(),
                        (point.0.position, point.1),
                        (points[i + 1].0.position, point.1 + 1),
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
                            (point.0.position, point.1),
                            (
                                kmp.itpt.entries[start_index as usize].position,
                                start_index as usize,
                            ),
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

    p1: (Vec3, usize),
    p2: (Vec3, usize),
) {
    let mut line_transform =
        Transform::from_translation(p1.0.lerp(p2.0, 0.5)).looking_at(p2.0, Vec3::Y);
    line_transform.scale.y = p1.0.distance(p2.0);
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
        ItptArrowLine {
            p1: p1.1,
            p2: p2.1,
            is_line: true,
        },
    ));

    let mut arrowhead_transform =
        Transform::from_translation(p1.0.lerp(p2.0, 0.5)).looking_at(p2.0, Vec3::Y);
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
        ItptArrowLine {
            p1: p1.1,
            p2: p2.1,
            is_line: false,
        },
    ));
}

#[allow(clippy::type_complexity)]
fn update_itpt(
    mut itpt: ParamSet<(
        Query<(&Transform, &ItptModel)>,
        Query<(&mut Transform, &ItptArrowLine)>,
    )>,
    kmp: Option<ResMut<Kmp>>,
) {
    if let Some(mut kmp) = kmp {
        for point in itpt.p0().iter() {
            kmp.itpt.entries[point.1 .0].position = point.0.translation;
        }
        for (mut transform, arrow_line) in itpt.p1().iter_mut() {
            let p1 = kmp.itpt.entries[arrow_line.p1].position;
            let p2 = kmp.itpt.entries[arrow_line.p2].position;

            *transform = Transform::from_translation(p1.lerp(p2, 0.5)).looking_at(p2, Vec3::Y);

            if arrow_line.is_line {
                transform.scale.y = p1.distance(p2);
                transform.rotate_local_x(f32::to_radians(90.));
            } else {
                transform.rotate_local_x(f32::to_radians(-90.));
            }
        }
    }
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

        let scale_before = transform.scale;

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
