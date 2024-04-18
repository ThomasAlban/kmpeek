use crate::{
    ui::{settings::AppSettings, update_ui::KclFileSelected},
    util::kcl_file::Kcl,
};
use bevy::{
    prelude::*,
    render::{mesh::PrimitiveTopology, render_asset::RenderAssetUsages, render_resource::Face},
};

use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, fs::File};

pub struct KclPlugin;

impl Plugin for KclPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KclModelUpdated>().add_systems(
            Update,
            (spawn_model.run_if(on_event::<KclFileSelected>()), update_kcl_model),
        );
    }
}

#[derive(Event, Default)]
pub struct KclModelUpdated;

#[derive(Resource, Serialize, Deserialize, Clone, PartialEq)]
pub struct KclModelSettings {
    pub visible: [bool; 32],
    pub color: [Color; 32],
    pub backface_culling: bool,
}
impl Default for KclModelSettings {
    fn default() -> Self {
        Self {
            visible: [true; 32],
            color: [
                Color::rgba(1.0, 1.0, 1.0, 1.0), // road
                Color::rgba(1.0, 0.9, 0.8, 1.0), // slippery road (sand/dirt)
                Color::rgba(0.0, 0.8, 0.0, 1.0), // weak off-road
                Color::rgba(0.0, 0.6, 0.0, 1.0), // off-road
                Color::rgba(0.0, 0.4, 0.0, 1.0), // heavy off-road
                Color::rgba(0.8, 0.9, 1.0, 1.0), // slippery road (ice)
                Color::rgba(1.0, 0.5, 0.0, 1.0), // boost panel
                Color::rgba(1.0, 0.6, 0.0, 1.0), // boost ramp
                Color::rgba(1.0, 0.8, 0.0, 1.0), // slow ramp
                Color::rgba(0.9, 0.9, 1.0, 0.5), // item road
                Color::rgba(0.7, 0.1, 0.1, 1.0), // solid fall
                Color::rgba(0.0, 0.5, 1.0, 1.0), // moving water
                Color::rgba(0.6, 0.6, 0.6, 1.0), // wall
                Color::rgba(0.0, 0.0, 0.6, 0.8), // invisible wall
                Color::rgba(0.6, 0.6, 0.7, 0.5), // item wall
                Color::rgba(0.6, 0.6, 0.6, 1.0), // wall
                Color::rgba(0.8, 0.0, 0.0, 0.8), // fall boundary
                Color::rgba(1.0, 0.0, 0.5, 0.8), // cannon activator
                Color::rgba(0.5, 0.0, 1.0, 0.5), // force recalculation
                Color::rgba(0.0, 0.3, 1.0, 1.0), // half-pipe ramp
                Color::rgba(0.6, 0.6, 0.6, 1.0), // wall (items pass through)
                Color::rgba(0.9, 0.9, 1.0, 1.0), // moving road
                Color::rgba(0.9, 0.7, 1.0, 1.0), // sticky road
                Color::rgba(1.0, 1.0, 1.0, 1.0), // road (alt sfx)
                Color::rgba(1.0, 0.0, 1.0, 0.8), // sound trigger
                Color::rgba(1.0, 0.0, 1.0, 0.5), // item state modifier
                Color::rgba(0.4, 0.6, 0.4, 0.8), // weak wall
                Color::rgba(0.9, 0.9, 1.0, 1.0), // rotating road
                Color::rgba(0.8, 0.0, 1.0, 0.8), // effect trigger
                Color::rgba(0.6, 0.6, 0.6, 1.0), // invisible wall 2
                Color::rgba(0.0, 0.6, 0.0, 0.8), // half-pipe invis wall
                Color::rgba(0.8, 0.7, 0.8, 1.0), // special wall
            ],
            backface_culling: false,
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
    mut q_model: Query<Entity, With<KCLModelSection>>,
    mut ev_kcl_file_selected: EventReader<KclFileSelected>,
    settings: Res<AppSettings>,
) {
    let Some(ev) = ev_kcl_file_selected.read().next() else {
        return;
    };
    if ev.0.extension() != Some(OsStr::new("kcl")) {
        return;
    }
    // despawn all entities with KCLModelSection (so that we have a clean slate)
    for entity in q_model.iter_mut() {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<Kcl>();

    // open the KCL file and read it
    let kcl_file = File::open(ev.0.clone()).expect("could not open kcl file");
    let kcl = Kcl::read(kcl_file).expect("could not read kcl file");
    // spawn the KCL model
    for i in 0..32 {
        let vertex_group = kcl.vertex_groups[i].clone();

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertex_group.vertices.clone());
        mesh.compute_flat_normals();

        let color = settings.kcl_model.color[i];

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(mesh),
                material: materials.add(StandardMaterial {
                    base_color: color,
                    cull_mode: if settings.kcl_model.backface_culling {
                        Some(Face::Back)
                    } else {
                        None
                    },
                    double_sided: !settings.kcl_model.backface_culling,
                    alpha_mode: if color.a() < 1. {
                        AlphaMode::Blend
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
            KCLModelSection(i),
        ));
    }
    commands.insert_resource(kcl);
}

pub fn update_kcl_model(
    mut q_kcl: Query<(&mut Visibility, &KCLModelSection, &mut Handle<StandardMaterial>), With<KCLModelSection>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<AppSettings>,
    mut ev_kcl_model_updated: EventReader<KclModelUpdated>,
) {
    // don't run this function unless the kcl model needs to be updated
    if ev_kcl_model_updated.is_empty() {
        return;
    } else {
        ev_kcl_model_updated.clear();
    }

    for (mut visibility, kcl_model_section, standard_material) in q_kcl.iter_mut() {
        let i = kcl_model_section.0;
        *visibility = if settings.kcl_model.visible[i] {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let material = materials.get_mut(standard_material.id()).unwrap();
        material.base_color = settings.kcl_model.color[i];
        material.alpha_mode = if material.base_color.a() < 1. {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        };
        material.cull_mode = if settings.kcl_model.backface_culling {
            Some(Face::Back)
        } else {
            None
        };
        material.double_sided = !settings.kcl_model.backface_culling
    }
}
