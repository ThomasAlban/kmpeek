use super::{
    checkpoints::CheckpointSpawner,
    meshes_materials::{KmpMaterials, PathMaterials, PointMaterials},
    path::{KmpPathNode, PathPointSpawner},
    point::PointSpawner,
    Ckpt, Cnpt, Jgpt, Mspt,
};
use crate::util::kmp_file::{Area, Came, Enpt, Gobj, Itpt, Ktpt, Poti, PotiPoint, Stgi};
use bevy::{math::vec3, prelude::*};
use std::collections::HashSet;
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Component, Default, Clone, Copy)]
pub struct TransformEditOptions {
    pub hide_rotation: bool,
    pub hide_y_translation: bool,
}

#[derive(Component, Default)]
pub struct KmpSelectablePoint;

// components attached to kmp entities, to store data about them:

// --- GENERAL PATH COMPONENTS ---
#[derive(Component, Default)]
pub struct PathStart;
#[derive(Component, Default)]
pub struct PathOverallStart;

// --- TRACK INFO COMPONENTS ---
#[derive(Resource, Default)]
pub struct TrackInfo {
    pub track_type: TrackType,
    pub lap_count: u8,
    pub speed_mod: f32,
    pub lens_flare_color: [u8; 4],
    pub lens_flare_flashing: bool,
    pub first_player_pos: FirstPlayerPos,
    pub narrow_player_spacing: bool,
}
#[derive(Default, Display, EnumIter, EnumString, IntoStaticStr, PartialEq, Clone)]
pub enum TrackType {
    #[default]
    Race,
    Battle,
}
#[derive(Default, Display, EnumIter, EnumString, IntoStaticStr, PartialEq, Clone)]
pub enum FirstPlayerPos {
    #[default]
    Left,
    Right,
}

// --- START POINT COMPONENTS ---
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub struct StartPoint {
    pub player_index: i16,
}
impl Default for StartPoint {
    fn default() -> Self {
        Self { player_index: -1 }
    }
}

// --- ENEMY PATH COMPONENTS ---
#[derive(Component, Clone, Copy, PartialEq, Default, Debug)]
pub struct EnemyPathPoint {
    pub leniency: f32,
    pub setting_1: EnemyPathSetting1,
    pub setting_2: EnemyPathSetting2,
    pub setting_3: u8,
    pub path_start_override: bool,
}
#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default, PartialEq, Clone, Copy, Debug)]
pub enum EnemyPathSetting1 {
    #[default]
    None,
    #[strum(serialize = "Requires Mushroom")]
    RequiresMushroom,
    #[strum(serialize = "Use Mushroom")]
    UseMushroom,
    Wheelie,
    #[strum(serialize = "End Wheelie")]
    EndWheelie,
}
#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default, PartialEq, Clone, Copy, Debug)]
pub enum EnemyPathSetting2 {
    #[default]
    None,
    #[strum(serialize = "End Drift")]
    EndDrift,
    #[strum(serialize = "Forbid Drift (?)")]
    ForbidDrift,
    #[strum(serialize = "Force Drift")]
    ForceDrift,
}

// --- ITEM PATH COMPONENTS ---
#[derive(Component, PartialEq, Clone, Default, Debug)]
pub struct ItemPathPoint {
    pub bullet_control: f32,
    pub bullet_height: ItemPathBulletHeight,
    pub bullet_cant_drop: bool,
    pub low_shell_priority: bool,
    pub path_start_override: bool,
}

#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default, PartialEq, Clone, Copy, Debug)]
pub enum ItemPathBulletHeight {
    #[default]
    Auto,
    #[strum(serialize = "Ignore Point Height")]
    IgnorePointHeight,
    #[strum(serialize = "Follow Point Height")]
    FollowPointHeight,
    #[strum(serialize = "Mushroom Pads (?)")]
    MushroomPads,
}

// --- CHECKPOINT COMPONENTS ---
// for checkpoints, the left checkpoint entity stores all the info
#[derive(Component, Clone, PartialEq, Debug)]
pub struct CheckpointLeft {
    pub right: Entity,
    pub line: Entity,
    pub plane: Entity,
    pub kind: CheckpointKind,
    pub path_start_override: bool,
    // will contain link to respawn entity
}
impl Default for CheckpointLeft {
    fn default() -> Self {
        Self {
            right: Entity::PLACEHOLDER,
            line: Entity::PLACEHOLDER,
            plane: Entity::PLACEHOLDER,
            kind: CheckpointKind::default(),
            path_start_override: false,
        }
    }
}
#[derive(Component, Clone, PartialEq)]
pub struct CheckpointRight {
    pub left: Entity,
    pub line: Entity,
    pub plane: Entity,
}
impl Default for CheckpointRight {
    fn default() -> Self {
        Self {
            left: Entity::PLACEHOLDER,
            line: Entity::PLACEHOLDER,
            plane: Entity::PLACEHOLDER,
        }
    }
}
#[derive(Component)]
pub struct CheckpointLine {
    pub left: Entity,
    pub right: Entity,
}
#[derive(Component)]
pub struct CheckpointPlane {
    pub left: Entity,
    pub right: Entity,
}

#[derive(Component, PartialEq, Clone, Default, Debug, Display, EnumString, IntoStaticStr, EnumIter)]
pub enum CheckpointKind {
    #[default]
    Normal,
    Key,
    #[strum(serialize = "Lap Count")]
    LapCount,
}

// --- OBJECT COMPONENTS ---
#[derive(Component, Default, Clone, PartialEq)]
pub struct Object {
    pub object_id: u16,
    pub scale: Vec3,
    pub route: u16,
    pub settings: [u16; 8],
    pub presence: u16,
}

// --- ROUTE COMPONENTS ---
#[derive(Component)]
pub struct Route {
    pub setting_1: u8,
    pub setting_2: u8,
}

#[derive(Component, Default)]
pub struct RouteMarker;
#[derive(Component)]
pub struct RoutePoint {
    pub setting_1: u16,
    pub setting_2: u16,
}

// --- AREA COMPONENTS ---
#[derive(Component, Clone, PartialEq)]
pub struct AreaPoint {
    pub shape: AreaShape,
    pub kind: AreaKind,
    pub priority: u8,
    pub scale: Vec3,
    pub show_area: bool,
}
impl Default for AreaPoint {
    fn default() -> Self {
        Self {
            shape: AreaShape::default(),
            kind: AreaKind::default(),
            priority: 0,
            scale: vec3(10000., 10000., 10000.),
            show_area: false,
        }
    }
}
#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default, Clone, PartialEq)]
pub enum AreaShape {
    #[default]
    Box,
    Cylinder,
}
#[derive(Display, EnumString, IntoStaticStr, EnumIter, Clone, PartialEq)]
pub enum AreaKind {
    Camera(AreaCameraIndex),
    #[strum(serialize = "Env Effect")]
    EnvEffect(AreaEnvEffectObject),
    #[strum(serialize = "Fog Effect")]
    FogEffect(AreaBfgEntry, AreaSetting2),
    #[strum(serialize = "Moving Road")]
    MovingRoad(AreaRouteId),
    #[strum(serialize = "Force Recalc")]
    ForceRecalc,
    #[strum(serialize = "Minimap Control")]
    MinimapControl(AreaSetting1, AreaSetting2),
    #[strum(serialize = "Bloom Effect")]
    BloomEffect(AreaBblmFile, AreaFadeTime),
    #[strum(serialize = "Enable Boos")]
    EnableBoos,
    #[strum(serialize = "Object Group")]
    ObjectGroup(AreaGroupId),
    #[strum(serialize = "Object Unload")]
    ObjectUnload(AreaGroupId),
    #[strum(serialize = "Fall Boundary")]
    FallBoundary,
}
impl Default for AreaKind {
    fn default() -> Self {
        Self::Camera(AreaCameraIndex(0))
    }
}
#[derive(Default, Clone, PartialEq)]
pub struct AreaCameraIndex(pub u8);
#[derive(Default, Clone, PartialEq, Display, EnumString, IntoStaticStr, EnumIter)]
pub enum AreaEnvEffectObject {
    #[default]
    EnvKareha,
    EnvKarehaUp,
}
#[derive(Default, Clone, PartialEq)]
pub struct AreaBfgEntry(pub u16);
#[derive(Default, Clone, PartialEq)]
pub struct AreaSetting1(pub u16);
#[derive(Default, Clone, PartialEq)]
pub struct AreaSetting2(pub u16);
#[derive(Default, Clone, PartialEq)]
pub struct AreaRouteId(pub u8);
#[derive(Default, Clone, PartialEq)]
pub struct AreaBblmFile(pub u16);
#[derive(Default, Clone, PartialEq)]
pub struct AreaFadeTime(pub u16);
#[derive(Default, Clone, PartialEq)]
pub struct AreaGroupId(pub u16);

// --- CAMERA COMPONENTS ---
#[derive(Component, Default, Clone, PartialEq)]
pub struct KmpCamera {
    pub kind: KmpCameraKind,
    pub next_index: u8,
    pub shake: u8,
    pub route: u8,
    pub point_velocity: u16,
    pub zoom_velocity: u16,
    pub view_velocity: u16,
    pub start: u8,
    pub movie: u8,
    pub zoom_start: f32,
    pub zoom_end: f32,
    pub view_start: Vec3,
    pub view_end: Vec3,
    pub time: f32,
}
#[derive(Default, Clone, PartialEq, Display, EnumString, IntoStaticStr, EnumIter)]
pub enum KmpCameraKind {
    #[default]
    Goal,
    FixSearch,
    PathSearch,
    KartFollow,
    KartPathFollow,
    #[allow(non_camel_case_types)]
    OP_FixMoveAt,
    #[allow(non_camel_case_types)]
    OP_PathMoveAt,
    MiniGame,
    MissionSuccess,
    Unknown,
}

// --- RESPAWN POINT COMPONENTS ---
#[derive(Component, Default, Clone, PartialEq)]
pub struct RespawnPoint {
    pub id: u16,
    pub sound_trigger: i8,
}

// --- CANNON POINT COMPONENTS
#[derive(Component, Default, Clone, PartialEq)]
pub struct CannonPoint {
    pub id: u16,
    pub shoot_effect: CannonShootEffect,
}
#[derive(Default, Display, EnumIter, EnumString, IntoStaticStr, PartialEq, Clone)]
pub enum CannonShootEffect {
    #[default]
    Straight,
    Curved,
    #[strum(serialize = "Curved & Slow")]
    CurvedSlow,
}

#[derive(Component, Default, Clone, PartialEq)]
pub struct BattleFinishPoint {
    pub id: u16,
}

//
// --- CONVERT COMPONENTS FROM KMP STORAGE FORMAT ---
//

#[derive(Clone)]
pub struct KmpError {
    #[allow(unused)]
    message: String,
}
impl KmpError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub trait FromKmp<T> {
    fn from_kmp(data: &T, errors: &mut Vec<KmpError>, index: usize) -> Self;
}

impl FromKmp<Stgi> for TrackInfo {
    fn from_kmp(data: &Stgi, errors: &mut Vec<KmpError>, _: usize) -> Self {
        Self {
            track_type: TrackType::Race,
            lap_count: data.lap_count,
            speed_mod: 0.,
            lens_flare_color: data.flare_color,
            lens_flare_flashing: data.lens_flare_flashing == 1,
            first_player_pos: match data.pole_pos {
                0 => FirstPlayerPos::Left,
                1 => FirstPlayerPos::Right,
                _ => {
                    errors.push(KmpError::new("Invalid STGI First Player Pos found"));
                    FirstPlayerPos::default()
                }
            },
            narrow_player_spacing: data.driver_distance == 1,
        }
    }
}
impl FromKmp<Ktpt> for StartPoint {
    fn from_kmp(data: &Ktpt, _: &mut Vec<KmpError>, _: usize) -> Self {
        Self {
            player_index: data.player_index,
        }
    }
}
impl FromKmp<Enpt> for EnemyPathPoint {
    fn from_kmp(data: &Enpt, errors: &mut Vec<KmpError>, _: usize) -> Self {
        Self {
            leniency: data.leniency,
            setting_1: match data.setting_1 {
                0 => EnemyPathSetting1::None,
                1 => EnemyPathSetting1::RequiresMushroom,
                2 => EnemyPathSetting1::UseMushroom,
                3 => EnemyPathSetting1::Wheelie,
                4 => EnemyPathSetting1::EndWheelie,
                _ => {
                    errors.push(KmpError::new("Invalid ENPT setting 1 found"));
                    EnemyPathSetting1::default()
                }
            },
            setting_2: match data.setting_2 {
                0 => EnemyPathSetting2::None,
                1 => EnemyPathSetting2::EndDrift,
                2 => EnemyPathSetting2::ForbidDrift,
                3 => EnemyPathSetting2::ForceDrift,
                _ => {
                    errors.push(KmpError::new("Invalid ENPT setting 2 found"));
                    EnemyPathSetting2::default()
                }
            },
            setting_3: data.setting_3,
            path_start_override: false,
        }
    }
}
impl FromKmp<Itpt> for ItemPathPoint {
    fn from_kmp(data: &Itpt, errors: &mut Vec<KmpError>, _: usize) -> Self {
        Self {
            bullet_control: data.bullet_control,
            bullet_height: match data.setting_1 {
                0 => ItemPathBulletHeight::IgnorePointHeight,
                1 => ItemPathBulletHeight::Auto,
                2 => ItemPathBulletHeight::FollowPointHeight,
                3 => ItemPathBulletHeight::MushroomPads,
                _ => {
                    errors.push(KmpError::new("Invalid ITPT setting 1 found"));
                    ItemPathBulletHeight::default()
                }
            },
            bullet_cant_drop: data.setting_2 == 1 || data.setting_2 == 3 || data.setting_1 == 5 || data.setting_2 == 7,
            low_shell_priority: data.setting_2 == 2
                || data.setting_2 == 3
                || data.setting_2 == 6
                || data.setting_2 == 7,
            path_start_override: false,
        }
    }
}
impl FromKmp<Ckpt> for CheckpointLeft {
    fn from_kmp(data: &Ckpt, errors: &mut Vec<KmpError>, _: usize) -> Self {
        Self {
            right: Entity::PLACEHOLDER,
            line: Entity::PLACEHOLDER,
            plane: Entity::PLACEHOLDER,
            kind: match data.cp_type {
                -1 => CheckpointKind::Normal,
                0 => CheckpointKind::LapCount,
                1..=127 => CheckpointKind::Key,
                _ => {
                    errors.push(KmpError::new("Invalid CKPT setting found"));
                    CheckpointKind::Normal
                }
            },
            path_start_override: false,
        }
    }
}
impl FromKmp<Gobj> for Object {
    fn from_kmp(data: &Gobj, _: &mut Vec<KmpError>, _: usize) -> Self {
        Self {
            object_id: data.object_id,
            scale: data.scale.into(),
            route: data.route,
            settings: data.settings,
            presence: data.presence_flags,
        }
    }
}
impl FromKmp<Poti> for Route {
    fn from_kmp(data: &Poti, _: &mut Vec<KmpError>, _: usize) -> Self {
        Self {
            setting_1: data.setting_1,
            setting_2: data.setting_2,
        }
    }
}
impl FromKmp<PotiPoint> for RoutePoint {
    fn from_kmp(data: &PotiPoint, _: &mut Vec<KmpError>, _: usize) -> Self {
        Self {
            setting_1: data.setting_1,
            setting_2: data.setting_2,
        }
    }
}
impl FromKmp<Area> for AreaPoint {
    fn from_kmp(data: &Area, errors: &mut Vec<KmpError>, _: usize) -> Self {
        Self {
            shape: match data.shape {
                0 => AreaShape::Box,
                1 => AreaShape::Cylinder,
                _ => {
                    errors.push(KmpError::new("Invalid AREA shape found"));
                    AreaShape::Box
                }
            },
            priority: data.priority,
            scale: Vec3::from(data.scale) * vec3(5000., 10000., 5000.),
            kind: match data.kind {
                0 => AreaKind::Camera(AreaCameraIndex(data.came_index)),
                1 => AreaKind::EnvEffect(match data.setting_1 {
                    0 => AreaEnvEffectObject::EnvKareha,
                    1 => AreaEnvEffectObject::EnvKarehaUp,
                    _ => {
                        errors.push(KmpError::new("Invalid AREA env effect object found"));
                        AreaEnvEffectObject::EnvKareha
                    }
                }),
                2 => AreaKind::FogEffect(AreaBfgEntry(data.setting_1), AreaSetting2(data.setting_2)),
                3 => AreaKind::MovingRoad(AreaRouteId(data.enpt_id)),
                4 => AreaKind::ForceRecalc,
                5 => AreaKind::MinimapControl(AreaSetting1(data.setting_1), AreaSetting2(data.setting_2)),
                6 => AreaKind::BloomEffect(AreaBblmFile(data.setting_1), AreaFadeTime(data.setting_2)),
                7 => AreaKind::EnableBoos,
                8 => AreaKind::ObjectGroup(AreaGroupId(data.setting_1)),
                9 => AreaKind::ObjectUnload(AreaGroupId(data.setting_1)),
                10 => AreaKind::FallBoundary,
                _ => {
                    errors.push(KmpError::new("Invalid AREA type found"));
                    AreaKind::default()
                }
            },
            show_area: false,
        }
    }
}
impl FromKmp<Came> for KmpCamera {
    fn from_kmp(data: &Came, errors: &mut Vec<KmpError>, _: usize) -> Self {
        Self {
            kind: match data.kind {
                0 => KmpCameraKind::Goal,
                1 => KmpCameraKind::FixSearch,
                2 => KmpCameraKind::PathSearch,
                3 => KmpCameraKind::KartFollow,
                4 => KmpCameraKind::KartPathFollow,
                5 => KmpCameraKind::OP_FixMoveAt,
                6 => KmpCameraKind::OP_PathMoveAt,
                7 => KmpCameraKind::MiniGame,
                8 => KmpCameraKind::MissionSuccess,
                9 => KmpCameraKind::Unknown,
                _ => {
                    errors.push(KmpError::new("Invalid CAME type found"));
                    KmpCameraKind::Goal
                }
            },
            next_index: data.next_index,
            shake: data.shake,
            route: data.route,
            point_velocity: data.point_velocity,
            zoom_velocity: data.zoom_velocity,
            view_velocity: data.view_velocity,
            start: data.start,
            movie: data.movie,
            zoom_start: data.zoom_start,
            zoom_end: data.zoom_end,
            view_start: data.view_start.into(),
            view_end: data.view_end.into(),
            time: data.time,
        }
    }
}
impl FromKmp<Jgpt> for RespawnPoint {
    fn from_kmp(data: &Jgpt, _: &mut Vec<KmpError>, index: usize) -> Self {
        Self {
            id: index as u16,
            sound_trigger: if data.extra_data >= 0 {
                ((data.extra_data / 100) - 1) as i8
            } else {
                -1
            },
        }
    }
}
impl FromKmp<Cnpt> for CannonPoint {
    fn from_kmp(data: &Cnpt, errors: &mut Vec<KmpError>, index: usize) -> Self {
        Self {
            id: index as u16,
            shoot_effect: match data.shoot_effect {
                0 => CannonShootEffect::Straight,
                1 => CannonShootEffect::Curved,
                2 => CannonShootEffect::CurvedSlow,
                _ => {
                    errors.push(KmpError::new("Invalid CNPT type found"));
                    CannonShootEffect::Straight
                }
            },
        }
    }
}
impl FromKmp<Mspt> for BattleFinishPoint {
    fn from_kmp(_: &Mspt, _: &mut Vec<KmpError>, index: usize) -> Self {
        Self { id: index as u16 }
    }
}

//
// --- IMPLEMENT HOW TO SPAWN EACH COMPONENT AS DEFAULT ---
//

pub trait SpawnNewPoint {
    fn spawn(commands: &mut Commands, pos: Vec3) -> Entity;
}
pub trait SpawnNewWithId {
    fn spawn(commands: &mut Commands, pos: Vec3, id: usize) -> Entity;
}
pub trait SpawnNewPath {
    fn spawn(commands: &mut Commands, pos: Vec3, prev_nodes: HashSet<Entity>) -> Entity;
}
macro_rules! impl_spawn_new {
    ($ty:ty) => {
        impl SpawnNewPoint for $ty {
            fn spawn(commands: &mut Commands, pos: Vec3) -> Entity {
                PointSpawner::new(Self::default())
                    .pos(pos)
                    .spawn_command(commands)
            }
        }
    };
}
macro_rules! impl_spawn_new_path {
    ($ty:ty, $marker:ty) => {
        impl SpawnNewPath for $ty {
            fn spawn(commands: &mut Commands, pos: Vec3, prev_nodes: HashSet<Entity>) -> Entity {
                let entity = PathPointSpawner::<_>::new(Self::default())
                    .pos(pos)
                    .spawn_command(commands);
                commands.add(move |world: &mut World| {
                    for prev_entity in prev_nodes.iter() {
                        KmpPathNode::link_nodes(*prev_entity, entity, world);
                    }
                });
                entity
            }
        }
    };
}
macro_rules! impl_spawn_new_with_id {
    ($ty:ty) => {
        impl SpawnNewWithId for $ty {
            fn spawn(commands: &mut Commands, pos: Vec3, id: usize) -> Entity {
                #[allow(clippy::needless_update)]
                PointSpawner::new(Self {
                    id: id as u16,
                    ..default()
                })
                .pos(pos)
                .spawn_command(commands)
            }
        }
    };
}
impl_spawn_new!(StartPoint);
impl_spawn_new!(Object);
impl_spawn_new!(AreaPoint);
impl_spawn_new!(KmpCamera);
impl_spawn_new_path!(ItemPathPoint, ItemPathMarker);
impl_spawn_new_path!(EnemyPathPoint, EnemyPathMarker);
impl_spawn_new_with_id!(RespawnPoint);
impl_spawn_new_with_id!(CannonPoint);
impl_spawn_new_with_id!(BattleFinishPoint);

impl CheckpointLeft {
    pub fn spawn(commands: &mut Commands, pos: Vec3, prev_left_nodes: HashSet<Entity>) -> (Entity, Entity) {
        let (left, right) = CheckpointSpawner::new(Self::default())
            .single_3d_pos(pos)
            .spawn_command(commands);
        commands.add(move |world: &mut World| {
            for prev_left in prev_left_nodes {
                KmpPathNode::link_nodes(prev_left, left, world);
                let prev_right = world.entity(prev_left).get::<CheckpointLeft>().unwrap().right;
                KmpPathNode::link_nodes(prev_right, right, world);
            }
        });
        (left, right)
    }
}

//
// --- IMPLEMENT HOW TO GET THE RELEVANT MATERIALS SECTION FOR THE COMPONENTS ---
//

pub trait GetPointMaterialSection {
    fn get_materials(materials: &KmpMaterials) -> &PointMaterials;
}
macro_rules! impl_point_material_section {
    ($ty:ty, $s:ident) => {
        impl GetPointMaterialSection for $ty {
            fn get_materials(materials: &KmpMaterials) -> &PointMaterials {
                &materials.$s
            }
        }
    };
}
impl_point_material_section!(StartPoint, start_points);
impl_point_material_section!(RespawnPoint, respawn_points);
impl_point_material_section!(Object, objects);
impl_point_material_section!(AreaPoint, areas);
impl_point_material_section!(KmpCamera, cameras);
impl_point_material_section!(CannonPoint, cannon_points);
impl_point_material_section!(BattleFinishPoint, battle_finish_points);
pub trait GetPathMaterialSection {
    fn get_materials(materials: &KmpMaterials) -> &PathMaterials;
}
impl GetPathMaterialSection for EnemyPathPoint {
    fn get_materials(materials: &KmpMaterials) -> &PathMaterials {
        &materials.enemy_paths
    }
}
impl GetPathMaterialSection for ItemPathPoint {
    fn get_materials(materials: &KmpMaterials) -> &PathMaterials {
        &materials.item_paths
    }
}
