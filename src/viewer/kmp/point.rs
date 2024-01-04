use bevy::{math::vec3, prelude::*};

use crate::{
    util::kmp_file::{KmpData, KmpPositionPoint, KmpRotationPoint},
    viewer::normalize::Normalize,
};

use super::{FromKmp, KmpSection};

pub fn spawn_point_section<
    T: KmpData + KmpPositionPoint + KmpRotationPoint + Send + 'static + Clone,
    U: Component + FromKmp<T>,
>(
    commands: &mut Commands,
    kmp_node_entries: &[T],

    sphere_mesh: Handle<Mesh>,
    cylinder_mesh: Handle<Mesh>,
    cone_mesh: Handle<Mesh>,

    sphere_material: Handle<StandardMaterial>,
    line_material: Handle<StandardMaterial>,
    arrow_material: Handle<StandardMaterial>,
    up_arrow_material: Handle<StandardMaterial>,
) {
    for node in kmp_node_entries.iter() {
        let euler_rot = node.get_rotation();
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            euler_rot.x.to_radians(),
            euler_rot.y.to_radians(),
            euler_rot.z.to_radians(),
        );
        commands
            .spawn((
                PbrBundle {
                    mesh: sphere_mesh.clone(),
                    material: sphere_material.clone(),
                    transform: Transform::from_translation(node.get_position())
                        .with_rotation(rotation),
                    visibility: Visibility::Hidden,
                    ..default()
                },
                U::from_kmp(node.clone()),
                KmpSection,
                Normalize::new(200., 30., BVec3::TRUE),
            ))
            .with_children(|parent| {
                let line_length = 750.;
                let mut line_transform = Transform::from_scale(vec3(1., line_length, 1.));
                line_transform.translation.z = line_length / 2.;
                line_transform.rotate_x(90_f32.to_radians());
                parent.spawn(PbrBundle {
                    mesh: cylinder_mesh.clone(),
                    material: line_material.clone(),
                    transform: line_transform,
                    ..default()
                });

                let mut arrow_transform = Transform::from_translation(vec3(0., 0., line_length));
                arrow_transform.rotate_x(90_f32.to_radians());
                parent.spawn(PbrBundle {
                    mesh: cone_mesh.clone(),
                    material: arrow_material.clone(),
                    transform: arrow_transform,
                    ..default()
                });

                let up_arrow_transform =
                    Transform::from_translation(vec3(0., line_length * 0.75, 0.))
                        .with_scale(vec3(1., 2., 1.));
                parent.spawn(PbrBundle {
                    mesh: cone_mesh.clone(),
                    material: up_arrow_material.clone(),
                    transform: up_arrow_transform,
                    ..default()
                });
            });
    }
}
