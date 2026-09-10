use crate::{
    ui::{
        settings::AppSettings,
        update_ui::{FileLoadSet, KclFileSelected},
    },
    util::{kcl_file::Kcl, try_despawn},
};
use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology, prelude::*, render::render_resource::Face};

use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, fs::File};

pub fn kcl_plugin(app: &mut App) {
    app.add_message::<KclModelUpdated>().add_systems(
        Update,
        (
            spawn_model
                .run_if(on_message::<KclFileSelected>)
                .in_set(FileLoadSet::Load),
            update_kcl_model,
        ),
    );
}

#[derive(Message, Default)]
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
                Color::srgba(1.0, 1.0, 1.0, 1.0), // road
                Color::srgba(1.0, 0.9, 0.8, 1.0), // slippery road (sand/dirt)
                Color::srgba(0.0, 0.8, 0.0, 1.0), // weak off-road
                Color::srgba(0.0, 0.6, 0.0, 1.0), // off-road
                Color::srgba(0.0, 0.4, 0.0, 1.0), // heavy off-road
                Color::srgba(0.8, 0.9, 1.0, 1.0), // slippery road (ice)
                Color::srgba(1.0, 0.5, 0.0, 1.0), // boost panel
                Color::srgba(1.0, 0.6, 0.0, 1.0), // boost ramp
                Color::srgba(1.0, 0.8, 0.0, 1.0), // slow ramp
                Color::srgba(0.9, 0.9, 1.0, 0.5), // item road
                Color::srgba(0.7, 0.1, 0.1, 1.0), // solid fall
                Color::srgba(0.0, 0.5, 1.0, 1.0), // moving water
                Color::srgba(0.6, 0.6, 0.6, 1.0), // wall
                Color::srgba(0.0, 0.0, 0.6, 0.8), // invisible wall
                Color::srgba(0.6, 0.6, 0.7, 0.5), // item wall
                Color::srgba(0.6, 0.6, 0.6, 1.0), // wall
                Color::srgba(0.8, 0.0, 0.0, 0.8), // fall boundary
                Color::srgba(1.0, 0.0, 0.5, 0.8), // cannon activator
                Color::srgba(0.5, 0.0, 1.0, 0.5), // force recalculation
                Color::srgba(0.0, 0.3, 1.0, 1.0), // half-pipe ramp
                Color::srgba(0.6, 0.6, 0.6, 1.0), // wall (items pass through)
                Color::srgba(0.9, 0.9, 1.0, 1.0), // moving road
                Color::srgba(0.9, 0.7, 1.0, 1.0), // sticky road
                Color::srgba(1.0, 1.0, 1.0, 1.0), // road (alt sfx)
                Color::srgba(1.0, 0.0, 1.0, 0.8), // sound trigger
                Color::srgba(1.0, 0.0, 1.0, 0.5), // item state modifier
                Color::srgba(0.4, 0.6, 0.4, 0.8), // weak wall
                Color::srgba(0.9, 0.9, 1.0, 1.0), // rotating road
                Color::srgba(0.8, 0.0, 1.0, 0.8), // effect trigger
                Color::srgba(0.6, 0.6, 0.6, 1.0), // invisible wall 2
                Color::srgba(0.0, 0.6, 0.0, 0.8), // half-pipe invis wall
                Color::srgba(0.8, 0.7, 0.8, 1.0), // special wall
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
    mut ev_kcl_file_selected: MessageReader<KclFileSelected>,
    settings: Res<AppSettings>,
) {
    let Some(ev) = ev_kcl_file_selected.read().next() else {
        return;
    };
    if ev.0.extension() != Some(OsStr::new("kcl")) {
        return;
    }
    // Parse the replacement before removing the current model. A missing or
    // malformed file should leave the editor's existing collision model intact.
    let kcl_file = match File::open(&ev.0) {
        Ok(file) => file,
        Err(error) => {
            error!("could not open KCL file {}: {error}", ev.0.display());
            return;
        }
    };
    let kcl = match Kcl::read(kcl_file) {
        Ok(kcl) => kcl,
        Err(error) => {
            error!("could not read KCL file {}: {error}", ev.0.display());
            return;
        }
    };

    // Parsing succeeded, so replace the old model.
    for entity in q_model.iter_mut() {
        try_despawn(&mut commands, entity);
    }
    commands.remove_resource::<Kcl>();

    // spawn the KCL model
    for i in 0..32 {
        let vertex_group = kcl.vertex_groups[i].clone();
        if vertex_group.vertices.is_empty() {
            continue;
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertex_group.vertices.clone());
        mesh.compute_flat_normals();

        let color = settings.kcl_model.color[i];

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                cull_mode: if settings.kcl_model.backface_culling {
                    Some(Face::Back)
                } else {
                    None
                },
                double_sided: !settings.kcl_model.backface_culling,
                alpha_mode: if color.alpha() < 1. {
                    AlphaMode::Blend
                } else {
                    AlphaMode::Opaque
                },
                ..default()
            })),
            if settings.kcl_model.visible[i] {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
            KCLModelSection(i),
        ));
    }
    commands.insert_resource(kcl);
}

pub fn update_kcl_model(
    mut q_kcl: Query<(&mut Visibility, &KCLModelSection, &mut MeshMaterial3d<StandardMaterial>), With<KCLModelSection>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<AppSettings>,
    mut ev_kcl_model_updated: MessageReader<KclModelUpdated>,
) {
    // don't run this function unless the kcl model needs to be updated
    if ev_kcl_model_updated.is_empty() {
        return;
    } else {
        ev_kcl_model_updated.clear();
    }

    for (mut visibility, kcl_model_section, standard_material) in q_kcl.iter_mut() {
        let i = kcl_model_section.0;
        let (Some(visible), Some(color)) = (settings.kcl_model.visible.get(i), settings.kcl_model.color.get(i)) else {
            warn!("ignoring invalid KCL model section index {i}");
            continue;
        };
        *visibility = if *visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let Some(mut material) = materials.get_mut(standard_material.id()) else {
            continue;
        };
        material.base_color = *color;
        material.alpha_mode = if material.base_color.alpha() < 1. {
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
