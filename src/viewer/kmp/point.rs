use super::{
    components::RespawnPoint,
    meshes_materials::{KmpMeshes, PointMaterials},
    settings::OutlineSettings,
    FromKmp, KmpSelectablePoint,
};
use crate::{
    util::kmp_file::{Jgpt, KmpFile, KmpGetSection, KmpPositionPoint, KmpRotationPoint},
    viewer::normalize::Normalize,
};
use bevy::{math::vec3, prelude::*};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};
use std::sync::Arc;

pub fn spawn_point_section<
    T: KmpGetSection + KmpPositionPoint + KmpRotationPoint + Send + 'static + Clone,
    U: Component + FromKmp<T>,
>(
    commands: &mut Commands,
    kmp: Arc<KmpFile>,
    meshes: KmpMeshes,
    materials: PointMaterials,
    outline: OutlineSettings,
) -> Vec<Entity> {
    let node_entries = &T::get_section(kmp.as_ref()).entries;
    let mut entities = Vec::with_capacity(node_entries.len());

    for node in node_entries.iter() {
        let position: Vec3 = node.get_position().into();
        let euler_rot: Vec3 = node.get_rotation().into();
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            euler_rot.x.to_radians(),
            euler_rot.y.to_radians(),
            euler_rot.z.to_radians(),
        );
        let entity = spawn_point::<T, U>(
            commands,
            &meshes,
            &materials,
            position,
            rotation,
            U::from_kmp(node),
            &outline,
            false,
        );
        entities.push(entity);
    }
    entities
}

pub fn spawn_point<
    T: KmpGetSection + KmpPositionPoint + KmpRotationPoint + Send + 'static + Clone,
    U: Component + FromKmp<T>,
>(
    commands: &mut Commands,
    meshes: &KmpMeshes,
    materials: &PointMaterials,
    position: Vec3,
    rotation: Quat,
    kmp_component: U,
    outline: &OutlineSettings,
    visible: bool,
) -> Entity {
    let mut result = commands.spawn((
        PbrBundle {
            mesh: meshes.sphere.clone(),
            material: materials.point.clone(),
            transform: Transform::from_translation(position).with_rotation(rotation),
            visibility: if visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
            ..default()
        },
        kmp_component,
        KmpSelectablePoint,
        Normalize::new(200., 30., BVec3::TRUE),
        OutlineBundle {
            outline: OutlineVolume {
                visible: false,
                colour: outline.color,
                width: outline.width,
            },
            ..default()
        },
    ));
    result.with_children(|parent| {
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

        let up_arrow_transform = Transform::from_translation(vec3(0., line_length * 0.75, 0.))
            .with_scale(vec3(1., 2., 1.));
        parent.spawn(PbrBundle {
            mesh: meshes.cone.clone(),
            material: materials.up_arrow.clone(),
            transform: up_arrow_transform,
            ..default()
        });
    });
    result.id()
}

pub fn spawn_respawn_point_section(
    commands: &mut Commands,
    kmp: Arc<KmpFile>,
    meshes: KmpMeshes,
    materials: PointMaterials,
    outline: OutlineSettings,
) -> Vec<Entity> {
    let node_entries = &Jgpt::get_section(kmp.as_ref()).entries;
    let mut entities = Vec::with_capacity(node_entries.len());

    for node in node_entries.iter() {
        let position: Vec3 = node.position.into();
        let euler_rot: Vec3 = node.rotation.into();
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
        let mut result = commands.spawn((
            SpatialBundle {
                transform: Transform::from_translation(position).with_rotation(rotation),
                visibility: Visibility::Hidden,
                ..default()
            },
            RespawnPoint,
            KmpSelectablePoint,
        ));
        result.with_children(|parent| {
            // sphere
            parent
                .spawn((
                    PbrBundle {
                        mesh: meshes.sphere.clone(),
                        material: materials.point.clone(),
                        ..default()
                    },
                    Normalize::new(200., 30., BVec3::TRUE),
                    OutlineBundle {
                        outline: OutlineVolume {
                            visible: false,
                            colour: outline.color,
                            width: outline.width,
                        },
                        ..default()
                    },
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
        entities.push(result.id());
    }
    entities
}
