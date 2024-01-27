use super::settings::{PathColor, PointColor};
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
}
pub struct KmpMaterials {
    pub start_points: PointMaterials,
    pub enemy_paths: PathMaterials,
    pub item_paths: PathMaterials,
    pub checkpoints: (),
    pub respawn_points: PointMaterials,
    pub objects: PointMaterials,
    pub areas: PointMaterials,
    pub cameras: PointMaterials,
    pub cannon_points: (),
    pub battle_finish_points: (),
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

pub fn unlit_material(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
) -> Handle<StandardMaterial> {
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
        sphere: meshes.add(Mesh::from(shape::UVSphere {
            radius: 100.,
            ..default()
        })),
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
    };

    let sections = &settings.kmp_model.sections;

    let kmp_materials = KmpMaterials {
        start_points: PointMaterials::from_colors(&mut materials, &sections.color.start_points),
        enemy_paths: PathMaterials::from_colors(&mut materials, &sections.color.enemy_paths),
        item_paths: PathMaterials::from_colors(&mut materials, &sections.color.item_paths),
        checkpoints: (),
        respawn_points: PointMaterials::from_colors(&mut materials, &sections.color.respawn_points),
        objects: PointMaterials::from_colors(&mut materials, &sections.color.objects),
        areas: PointMaterials::from_colors(&mut materials, &sections.color.areas),
        cameras: PointMaterials::from_colors(&mut materials, &sections.color.cameras),
        cannon_points: (),
        battle_finish_points: (),
    };

    let kmp_meshes_materials = KmpMeshesMaterials {
        meshes: kmp_meshes,
        materials: kmp_materials,
    };

    commands.insert_resource(kmp_meshes_materials);
}
