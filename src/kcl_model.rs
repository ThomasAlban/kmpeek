use crate::{
    kcl_file::Kcl,
    ui::{AppSettings, KclFileSelected},
};
use bevy::{prelude::*, render::mesh::PrimitiveTopology};
use bevy_pkv::PkvStore;
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, fs::File};

pub struct KclPlugin;

impl Plugin for KclPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_model, update_kcl_model));
    }
}

#[derive(Resource, Serialize, Deserialize)]
pub struct KclModelSettings {
    pub visible: [bool; 32],
    pub color: [[f32; 4]; 32],
}
impl Default for KclModelSettings {
    fn default() -> Self {
        Self {
            visible: [true; 32],
            color: [
                [1.0, 1.0, 1.0, 1.0], // road
                [1.0, 0.9, 0.8, 1.0], // slippery road (sand/dirt)
                [0.0, 0.8, 0.0, 1.0], // weak off-road
                [0.0, 0.6, 0.0, 1.0], // off-road
                [0.0, 0.4, 0.0, 1.0], // heavy off-road
                [0.8, 0.9, 1.0, 1.0], // slippery road (ice)
                [1.0, 0.5, 0.0, 1.0], // boost panel
                [1.0, 0.6, 0.0, 1.0], // boost ramp
                [1.0, 0.8, 0.0, 1.0], // slow ramp
                [0.9, 0.9, 1.0, 0.5], // item road
                [0.7, 0.1, 0.1, 1.0], // solid fall
                [0.0, 0.5, 1.0, 1.0], // moving water
                [0.6, 0.6, 0.6, 1.0], // wall
                [0.0, 0.0, 0.6, 0.8], // invisible wall
                [0.6, 0.6, 0.7, 0.5], // item wall
                [0.6, 0.6, 0.6, 1.0], // wall
                [0.8, 0.0, 0.0, 0.8], // fall boundary
                [1.0, 0.0, 0.5, 0.8], // cannon activator
                [0.5, 0.0, 1.0, 0.5], // force recalculation
                [0.0, 0.3, 1.0, 1.0], // half-pipe ramp
                [0.6, 0.6, 0.6, 1.0], // wall (items pass through)
                [0.9, 0.9, 1.0, 1.0], // moving road
                [0.9, 0.7, 1.0, 1.0], // sticky road
                [1.0, 1.0, 1.0, 1.0], // road (alt sfx)
                [1.0, 0.0, 1.0, 0.8], // sound trigger
                [1.0, 0.0, 1.0, 0.5], // item state modifier
                [0.4, 0.6, 0.4, 0.8], // weak wall
                [0.9, 0.9, 1.0, 1.0], // rotating road
                [0.8, 0.0, 1.0, 0.8], // effect trigger
                [0.6, 0.6, 0.6, 1.0], // invisible wall 2
                [0.0, 0.6, 0.0, 0.8], // half-pipe invis wall
                [0.8, 0.7, 0.8, 1.0], // special wall
            ],
        }
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
    mut ev_kcl_file_selected: EventReader<KclFileSelected>,
    pkv: Res<PkvStore>,
) {
    let settings = pkv
        .get::<AppSettings>("settings")
        .expect("could not get user settings");
    for ev in ev_kcl_file_selected.iter() {
        if ev.0.extension() != Some(OsStr::new("kcl")) {
            continue;
        }
        // despawn all entities with KCLModelSection (so that we have a clean slate)
        for entity in model.iter_mut() {
            commands.entity(entity).despawn();
        }
        commands.remove_resource::<Kcl>();

        // open the KCL file and read it
        let kcl_file = File::open(ev.0.clone()).unwrap();
        let kcl = Kcl::read(kcl_file).unwrap();
        // spawn the KCL model
        for i in 0..32 {
            let vertex_group = kcl.vertex_groups[i].clone();
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertex_group.vertices.clone());
            mesh.compute_flat_normals();

            let color: Color = settings.kcl_model.color[i].into();

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
                    visibility: if settings.kcl_model.visible[i] {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                    ..default()
                },
            ));
        }
        commands.insert_resource(kcl);
    }
}

pub fn update_kcl_model(
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
    pkv: Res<PkvStore>,
) {
    let settings = pkv
        .get::<AppSettings>("settings")
        .expect("could not get user settings");
    for (mut visibility, kcl_model_section, standard_material, _) in query.iter_mut() {
        let i = kcl_model_section.0;
        *visibility = if settings.kcl_model.visible[i] {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let material = materials.get_mut(&standard_material).unwrap();
        material.base_color = settings.kcl_model.color[i].into();
        material.alpha_mode = if material.base_color.a() < 1. {
            AlphaMode::Add
        } else {
            AlphaMode::Opaque
        };
    }
}
