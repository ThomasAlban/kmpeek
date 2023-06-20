use crate::{kcl_file::Kcl, ui::FileSelected};
use bevy::{prelude::*, render::mesh::PrimitiveTopology};
use std::fs::File;

pub struct KclPlugin;

impl Plugin for KclPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_model).add_system(update_kcl_model);
    }
}

// this is a component attached to every part of the KCL model so that we know which bit it is when querying
#[derive(Component)]
pub struct KCLModelSection(pub usize);

pub fn spawn_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut model: Query<Entity, With<KCLModelSection>>,
    mut ev_file_selected: EventReader<FileSelected>,
) {
    for ev in ev_file_selected.iter() {
        // despawn all entities with KCLModelSection (so that we have a clean slate)
        for entity in model.iter_mut() {
            commands.entity(entity).despawn();
        }
        // open the KCL file and read it
        let kcl_file = File::open(ev.0.clone()).unwrap();
        let kcl = Kcl::read(kcl_file).unwrap();
        // spawn the KCL model
        for i in 0..32 {
            let vertex_group = kcl.vertex_groups[i].clone();
            if vertex_group.visible {
                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertex_group.vertices.clone());
                mesh.compute_flat_normals();

                let color: Color = vertex_group.color.into();

                commands.spawn((
                    KCLModelSection(i),
                    PbrBundle {
                        mesh: meshes.add(mesh),
                        material: materials.add(StandardMaterial {
                            base_color: color,
                            cull_mode: None,
                            double_sided: true,
                            alpha_mode: if color.a() < 1. {
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
        commands.insert_resource(kcl);
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
        material.base_color = kcl.vertex_groups[i].color.into();
        material.alpha_mode = if material.base_color.a() < 1. {
            AlphaMode::Add
        } else {
            AlphaMode::Opaque
        };
    }
}
