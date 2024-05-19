use super::settings::{CheckpointColour, PathColor, PointColor};
use crate::{
    ui::settings::AppSettings,
    util::shapes::{Cone, Cylinder},
};
use bevy::prelude::*;

#[derive(Resource)]
pub struct KmpMeshesMaterials {
    pub meshes: KmpMeshes,
    pub materials: KmpMaterials,
}

#[derive(Clone)]
pub struct KmpMeshes {
    pub sphere: Handle<Mesh>,
    pub cylinder: Handle<Mesh>,
    pub frustrum: Handle<Mesh>,
    pub cone: Handle<Mesh>,
    pub plane: Handle<Mesh>,
}
pub struct KmpMaterials {
    pub start_points: PointMaterials,
    pub enemy_paths: PathMaterials,
    pub item_paths: PathMaterials,
    pub checkpoints: CheckpointMaterials,
    pub respawn_points: PointMaterials,
    pub objects: PointMaterials,
    pub areas: PointMaterials,
    pub cameras: PointMaterials,
    pub cannon_points: PointMaterials,
    pub battle_finish_points: PointMaterials,
}

#[derive(Clone)]
pub struct PathMaterials {
    pub point: Handle<StandardMaterial>,
    pub line: Handle<StandardMaterial>,
    pub arrow: Handle<StandardMaterial>,
}
impl PathMaterials {
    pub fn from_colors(materials: &mut Assets<StandardMaterial>, colors: &PathColor) -> Self {
        Self {
            point: unlit_material(materials, colors.point),
            line: unlit_material(materials, colors.line),
            arrow: unlit_material(materials, colors.arrow),
        }
    }
}

#[derive(Clone)]
pub struct PointMaterials {
    pub point: Handle<StandardMaterial>,
    pub line: Handle<StandardMaterial>,
    pub arrow: Handle<StandardMaterial>,
    pub up_arrow: Handle<StandardMaterial>,
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

#[derive(Clone)]
pub struct CheckpointMaterials {
    pub normal: Handle<StandardMaterial>,
    pub normal_plane: Handle<StandardMaterial>,
    pub key: Handle<StandardMaterial>,
    pub key_plane: Handle<StandardMaterial>,
    pub lap_count: Handle<StandardMaterial>,
    pub lap_count_plane: Handle<StandardMaterial>,
    pub line: Handle<StandardMaterial>,
    pub arrow: Handle<StandardMaterial>,
}
impl CheckpointMaterials {
    pub fn from_colors(materials: &mut Assets<StandardMaterial>, colors: &CheckpointColour) -> Self {
        let plane_color = |materials: &mut Assets<StandardMaterial>, color: Color| {
            materials.add(StandardMaterial {
                base_color: color.with_a(0.2),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                cull_mode: None,
                ..default()
            })
        };
        Self {
            normal: unlit_material(materials, colors.normal),
            normal_plane: plane_color(materials, colors.normal),
            key: unlit_material(materials, colors.key),
            key_plane: plane_color(materials, colors.key),
            lap_count: unlit_material(materials, colors.lap_count),
            lap_count_plane: plane_color(materials, colors.lap_count),
            line: unlit_material(materials, colors.line),
            arrow: unlit_material(materials, colors.arrow),
        }
    }
}

pub fn unlit_material(materials: &mut Assets<StandardMaterial>, color: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: if color.a() < 1. {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        unlit: true,
        ..default()
    })
}

pub fn setup_kmp_meshes_materials(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<AppSettings>,
) {
    let kmp_meshes = KmpMeshes {
        sphere: meshes.add(Sphere::new(100.).mesh()),
        cylinder: meshes.add(Mesh::from(Cylinder {
            height: 1.,
            radius_bottom: 50.,
            radius_top: 50.,
            radial_segments: 32,
            height_segments: 32,
        })),
        frustrum: meshes.add(Mesh::from(Cylinder {
            height: 100.,
            radius_bottom: 100.,
            radius_top: 50.,
            radial_segments: 32,
            height_segments: 32,
        })),
        cone: meshes.add(Mesh::from(Cone {
            height: 200.,
            radius: 100.,
            segments: 32,
        })),
        plane: meshes.add(Plane3d::default().mesh()),
    };

    let colors = &settings.kmp_model.color;

    let kmp_materials = KmpMaterials {
        start_points: PointMaterials::from_colors(&mut materials, &colors.start_points),
        enemy_paths: PathMaterials::from_colors(&mut materials, &colors.enemy_paths),
        item_paths: PathMaterials::from_colors(&mut materials, &colors.item_paths),
        checkpoints: CheckpointMaterials::from_colors(&mut materials, &colors.checkpoints),
        respawn_points: PointMaterials::from_colors(&mut materials, &colors.respawn_points),
        objects: PointMaterials::from_colors(&mut materials, &colors.objects),
        areas: PointMaterials::from_colors(&mut materials, &colors.areas),
        cameras: PointMaterials::from_colors(&mut materials, &colors.cameras),
        cannon_points: PointMaterials::from_colors(&mut materials, &colors.cannon_points),
        battle_finish_points: PointMaterials::from_colors(&mut materials, &colors.battle_finish_points),
    };

    let kmp_meshes_materials = KmpMeshesMaterials {
        meshes: kmp_meshes,
        materials: kmp_materials,
    };

    commands.insert_resource(kmp_meshes_materials);
}
