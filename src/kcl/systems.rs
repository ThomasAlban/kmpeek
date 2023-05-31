use super::{components::KCLModelSection, resources::Kcl};
use bevy::prelude::*;

use bevy::render::mesh::PrimitiveTopology;

pub fn spawn_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    kcl: Res<Kcl>,
) {
    for i in 0..32 {
        let vertex_group = kcl.vertex_groups[i].clone();
        if vertex_group.visible {
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertex_group.vertices.clone());
            mesh.compute_flat_normals();

            let colour: Color = vertex_group.colour.into();

            commands.spawn((
                KCLModelSection(i),
                PbrBundle {
                    mesh: meshes.add(mesh),
                    material: materials.add(StandardMaterial {
                        base_color: colour,
                        cull_mode: None,
                        double_sided: true,
                        alpha_mode: if colour.a() < 1. {
                            AlphaMode::Add
                        } else {
                            AlphaMode::Opaque
                        },
                        ..default()
                    }),
                    ..default()
                },
            ));
        }
    }
}

pub fn update_kcl_model(
    kcl: ResMut<Kcl>,
    mut query: Query<
        (
            &mut Visibility,
            &KCLModelSection,
            &mut Handle<StandardMaterial>,
            Entity,
        ),
        With<KCLModelSection>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !kcl.is_changed() {
        return;
    }

    for (mut visibility, kcl_model_section, standard_material, _) in query.iter_mut() {
        let i = kcl_model_section.0;
        *visibility = if kcl.vertex_groups[i].visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let material = materials.get_mut(&standard_material).unwrap();
        material.base_color = kcl.vertex_groups[i].colour.into();
        material.alpha_mode = if material.base_color.a() < 1. {
            AlphaMode::Add
        } else {
            AlphaMode::Opaque
        };
    }
}
