use std::marker::PhantomData;

use super::{
    settings::{CheckpointColor, PathColor, PointColor},
    AreaPoint, BattleFinishPoint, CannonPoint, EnemyPathPoint, ItemPathPoint, KmpCamera, Object, RespawnPoint,
    StartPoint,
};
use crate::{
    ui::settings::AppSettings,
    util::shapes::{Cone, Cylinder},
};
use bevy::prelude::*;

#[derive(Clone, Resource)]
pub struct KmpMeshes {
    pub sphere: Handle<Mesh>,
    pub cylinder: Handle<Mesh>,
    pub frustrum: Handle<Mesh>,
    pub cone: Handle<Mesh>,
    pub plane: Handle<Mesh>,
}

#[derive(Clone, Resource)]
pub struct PointMaterials<T: Component + Clone> {
    pub point: Handle<StandardMaterial>,
    pub line: Handle<StandardMaterial>,
    pub arrow: Handle<StandardMaterial>,
    pub up_arrow: Handle<StandardMaterial>,
    _p: PhantomData<T>,
}
#[derive(Clone, Resource)]
pub struct PathMaterials<T: Component + Clone> {
    pub point: Handle<StandardMaterial>,
    pub line: Handle<StandardMaterial>,
    pub arrow: Handle<StandardMaterial>,
    _p: PhantomData<T>,
}
#[derive(Clone, Resource)]
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

pub trait MaterialsFromColors<Colors> {
    fn from_colors(materials: &mut Assets<StandardMaterial>, colors: &Colors) -> Self;
}
impl<T: Component + Clone> MaterialsFromColors<PointColor> for PointMaterials<T> {
    fn from_colors(materials: &mut Assets<StandardMaterial>, colors: &PointColor) -> Self {
        Self {
            point: unlit_material(materials, colors.point),
            line: unlit_material(materials, colors.line),
            arrow: unlit_material(materials, colors.arrow),
            up_arrow: unlit_material(materials, colors.up_arrow),
            _p: PhantomData,
        }
    }
}
impl<T: Component + Clone> MaterialsFromColors<PathColor> for PathMaterials<T> {
    fn from_colors(materials: &mut Assets<StandardMaterial>, colors: &PathColor) -> Self {
        Self {
            point: unlit_material(materials, colors.point),
            line: unlit_material(materials, colors.line),
            arrow: unlit_material(materials, colors.arrow),
            _p: PhantomData,
        }
    }
}
impl MaterialsFromColors<CheckpointColor> for CheckpointMaterials {
    fn from_colors(materials: &mut Assets<StandardMaterial>, colors: &CheckpointColor) -> Self {
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
    commands.insert_resource(kmp_meshes);

    let colors = &settings.kmp_model.color;

    let start_points = PointMaterials::<StartPoint>::from_colors(&mut materials, &colors.start_points);
    commands.insert_resource(start_points);
    let enemy_paths = PathMaterials::<EnemyPathPoint>::from_colors(&mut materials, &colors.enemy_paths);
    commands.insert_resource(enemy_paths);
    let item_paths = PathMaterials::<ItemPathPoint>::from_colors(&mut materials, &colors.item_paths);
    commands.insert_resource(item_paths);
    let checkpoints = CheckpointMaterials::from_colors(&mut materials, &colors.checkpoints);
    commands.insert_resource(checkpoints);
    let respawn_points = PointMaterials::<RespawnPoint>::from_colors(&mut materials, &colors.respawn_points);
    commands.insert_resource(respawn_points);
    let objects = PointMaterials::<Object>::from_colors(&mut materials, &colors.objects);
    commands.insert_resource(objects);
    let areas = PointMaterials::<AreaPoint>::from_colors(&mut materials, &colors.areas);
    commands.insert_resource(areas);
    let cameras = PointMaterials::<KmpCamera>::from_colors(&mut materials, &colors.cameras);
    commands.insert_resource(cameras);
    let cannon_points = PointMaterials::<CannonPoint>::from_colors(&mut materials, &colors.cannon_points);
    commands.insert_resource(cannon_points);
    let battle_finish_points =
        PointMaterials::<BattleFinishPoint>::from_colors(&mut materials, &colors.battle_finish_points);
    commands.insert_resource(battle_finish_points);
}
