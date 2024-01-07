use std::sync::Arc;

use bevy::{math::vec3, prelude::*};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};

use crate::{
    util::kmp_file::{Jgpt, Kmp, KmpData, KmpSectionName, Section},
    viewer::normalize::Normalize,
};

use super::{components::RespawnPoint, settings::PointColor, unlit_material, FromKmp, KmpSection};

#[derive(Clone)]
pub struct PointMeshes {
    sphere: Handle<Mesh>,
    cylinder: Handle<Mesh>,
    cone: Handle<Mesh>,
}
impl PointMeshes {
    pub fn new(sphere: Handle<Mesh>, cylinder: Handle<Mesh>, cone: Handle<Mesh>) -> Self {
        Self {
            sphere,
            cylinder,
            cone,
        }
    }
}

pub struct PointMaterials {
    point: Handle<StandardMaterial>,
    line: Handle<StandardMaterial>,
    arrow: Handle<StandardMaterial>,
    up_arrow: Handle<StandardMaterial>,
}
impl PointMaterials {
    pub fn from_colors(materials: &mut Assets<StandardMaterial>, colors: &PointColor) -> Self {
        Self {
            point: unlit_material(materials, colors.point),
            line: unlit_material(materials, colors.line),
            arrow: unlit_material(materials, colors.arrow),
            up_arrow: unlit_material(materials, colors.up_arrow),
        }
    }
}

pub fn spawn_point_section<
    T: KmpData + KmpSectionName + Send + 'static + Clone + Reflect + TypePath + FromReflect + Struct,
    U: Component + FromKmp<T>,
>(
    commands: &mut Commands,
    kmp: Arc<Kmp>,
    meshes: PointMeshes,
    materials: PointMaterials,
) {
    let node_entries: &[T] = &kmp
        .get_field::<Section<T>>(&T::section_name())
        .unwrap()
        .entries;

    for node in node_entries.iter() {
        let position = node.get_field::<Vec3>("position").unwrap();
        let euler_rot = node.get_field::<Vec3>("rotation").unwrap();
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            euler_rot.x.to_radians(),
            euler_rot.y.to_radians(),
            euler_rot.z.to_radians(),
        );
        commands
            .spawn((
                PbrBundle {
                    mesh: meshes.sphere.clone(),
                    material: materials.point.clone(),
                    transform: Transform::from_translation(*position).with_rotation(rotation),
                    visibility: Visibility::Hidden,
                    ..default()
                },
                U::from_kmp(node),
                KmpSection,
                Normalize::new(200., 30., BVec3::TRUE),
                OutlineBundle {
                    outline: OutlineVolume {
                        visible: false,
                        colour: Color::rgba(1.0, 1.0, 1.0, 0.3),
                        width: 7.0,
                    },
                    ..default()
                },
            ))
            .with_children(|parent| {
                let line_length = 750.;
                let mut line_transform = Transform::from_scale(vec3(1., line_length, 1.));
                line_transform.translation.z = line_length / 2.;
                line_transform.rotate_x(90_f32.to_radians());
                parent.spawn(PbrBundle {
                    mesh: meshes.cylinder.clone(),
                    material: materials.line.clone(),
                    transform: line_transform,
                    ..default()
                });

                let mut arrow_transform = Transform::from_translation(vec3(0., 0., line_length));
                arrow_transform.rotate_x(90_f32.to_radians());
                parent.spawn(PbrBundle {
                    mesh: meshes.cone.clone(),
                    material: materials.arrow.clone(),
                    transform: arrow_transform,
                    ..default()
                });

                let up_arrow_transform =
                    Transform::from_translation(vec3(0., line_length * 0.75, 0.))
                        .with_scale(vec3(1., 2., 1.));
                parent.spawn(PbrBundle {
                    mesh: meshes.cone.clone(),
                    material: materials.up_arrow.clone(),
                    transform: up_arrow_transform,
                    ..default()
                });
            });
    }
}

pub fn spawn_respawn_point_section(
    commands: &mut Commands,
    kmp: Arc<Kmp>,
    meshes: PointMeshes,
    materials: PointMaterials,
) {
    let node_entries: &[Jgpt] = &kmp.get_field::<Section<Jgpt>>("jgpt").unwrap().entries;

    for node in node_entries.iter() {
        let position = node.get_field::<Vec3>("position").unwrap();
        let euler_rot = node.get_field::<Vec3>("rotation").unwrap();
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            euler_rot.x.to_radians(),
            euler_rot.y.to_radians(),
            euler_rot.z.to_radians(),
        );
        // parent child hierarchy is different here
        // a parent is spawed containing only a spatial bundle and is not normalized
        // there are 2 children - the main point, and the respawn previews
        // only the main point is normalized, and the main point has arrow and line children as normal
        // this is so that the respawn preview points don't get normalized
        commands
            .spawn((
                SpatialBundle {
                    transform: Transform::from_translation(*position).with_rotation(rotation),
                    visibility: Visibility::Hidden,
                    ..default()
                },
                RespawnPoint,
                KmpSection,
            ))
            .with_children(|parent| {
                // sphere
                parent
                    .spawn((
                        PbrBundle {
                            mesh: meshes.sphere.clone(),
                            material: materials.point.clone(),
                            ..default()
                        },
                        Normalize::new(200., 30., BVec3::TRUE),
                    ))
                    .with_children(|parent| {
                        // line
                        let line_length = 750.;
                        let mut line_transform = Transform::from_scale(vec3(1., line_length, 1.));
                        line_transform.translation.z = line_length / 2.;
                        line_transform.rotate_x(90_f32.to_radians());
                        parent.spawn(PbrBundle {
                            mesh: meshes.cylinder.clone(),
                            material: materials.line.clone(),
                            transform: line_transform,
                            ..default()
                        });

                        // arrow
                        let mut arrow_transform =
                            Transform::from_translation(vec3(0., 0., line_length));
                        arrow_transform.rotate_x(90_f32.to_radians());
                        parent.spawn(PbrBundle {
                            mesh: meshes.cone.clone(),
                            material: materials.arrow.clone(),
                            transform: arrow_transform,
                            ..default()
                        });

                        // up arrow
                        let up_arrow_transform =
                            Transform::from_translation(vec3(0., line_length * 0.75, 0.))
                                .with_scale(vec3(1., 2., 1.));
                        parent.spawn(PbrBundle {
                            mesh: meshes.cone.clone(),
                            material: materials.up_arrow.clone(),
                            transform: up_arrow_transform,
                            ..default()
                        });
                    });

                // spawn respawn position previews
                let y = 700.;
                let mut z = -600.;
                while z <= 0. {
                    let mut x = -450.;
                    while x <= 450. {
                        parent.spawn({
                            PbrBundle {
                                mesh: meshes.sphere.clone(),
                                material: materials.line.clone(),
                                transform: Transform::from_translation(vec3(x, y, z))
                                    .with_scale(Vec3::splat(0.5)),
                                ..default()
                            }
                        });
                        x += 300.;
                    }
                    z += 300.;
                }
            });
    }
}
