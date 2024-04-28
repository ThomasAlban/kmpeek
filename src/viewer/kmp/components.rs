use super::{
    meshes_materials::KmpMeshesMaterials,
    path::{KmpPathNode, PathPointSpawner},
    point::{add_respawn_point_preview, PointSpawner},
    settings::OutlineSettings,
    Ckpt, Cnpt, Jgpt, Mspt,
};
use crate::util::kmp_file::{Area, Came, Enpt, Gobj, Itpt, Ktpt, Poti, PotiPoint, Stgi};
use bevy::{math::vec3, prelude::*};
use std::collections::HashSet;
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Component)]
pub struct TransformOptions {
    /// whether or not the object's rotation should be editable
    pub use_rotation: bool,
    /// whether or not the object's scale should be editable
    pub use_scale: bool,
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
#[derive(Component, Default)]
pub struct EnemyPathMarker;
#[derive(Component, Clone, Copy, PartialEq, Default)]
pub struct EnemyPathPoint {
    pub leniency: f32,
    pub setting_1: EnemyPathSetting1,
    pub setting_2: EnemyPathSetting2,
    pub setting_3: u8,
    pub path_start_override: bool,
}
#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default, PartialEq, Clone, Copy)]
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
#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default, PartialEq, Clone, Copy)]
pub enum EnemyPathSetting2 {
    #[default]
    None,
    #[strum(serialize = "End Drift")]
    EndDrift,
    #[strum(serialize = "Forbid Drift (?)")]
    ForbidDrift,
    #[strum(serialize = "ForceDrift")]
    ForceDrift,
}

// --- ITEM PATH COMPONENTS ---
#[derive(Component, Default)]
pub struct ItemPathMarker;
#[derive(Component, PartialEq, Clone, Default)]
pub struct ItemPathPoint {
    pub bullet_control: f32,
    pub bullet_height: ItemPathBulletHeight,
    pub bullet_cant_drop: bool,
    pub low_shell_priority: bool,
    pub path_start_override: bool,
}

#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default, PartialEq, Clone, Copy)]
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
#[derive(Component, Clone, PartialEq)]
pub struct CheckpointLeft {
    pub right: Entity,
    pub kind: CheckpointKind,
    // will contain link to respawn entity
}
#[derive(Component, Clone, PartialEq)]
pub struct CheckpointRight {
    pub left: Entity,
}

#[derive(Component, PartialEq, Clone, Default)]
pub enum CheckpointKind {
    #[default]
    Normal,
    Key(u8),
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
            kind: match data.cp_type {
                -1 => CheckpointKind::Normal,
                0 => CheckpointKind::LapCount,
                x @ 1..=127 => CheckpointKind::Key(x as u8),
                _ => {
                    errors.push(KmpError::new("Invalid CKPT setting found"));
                    CheckpointKind::Normal
                }
            },
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

pub trait Spawnable {
    fn spawn(commands: &mut Commands, meshes_materials: &KmpMeshesMaterials, pos: Vec3) -> Entity;
}
macro_rules! impl_spawnable_point {
    ($ty:ty, $s:ident) => {
        impl Spawnable for $ty {
            fn spawn(commands: &mut Commands, meshes_materials: &KmpMeshesMaterials, pos: Vec3) -> Entity {
                PointSpawner::new(
                    &meshes_materials.meshes,
                    &meshes_materials.materials.$s,
                    &OutlineSettings::default(),
                    Self::default(),
                )
                .pos(pos)
                .spawn_command(commands)
            }
        }
    };
}
impl_spawnable_point!(StartPoint, start_points);
impl_spawnable_point!(Object, objects);
impl_spawnable_point!(AreaPoint, areas);
impl_spawnable_point!(KmpCamera, cameras);
impl RespawnPoint {
    pub fn spawn(commands: &mut Commands, meshes_materials: &KmpMeshesMaterials, pos: Vec3, id: usize) -> Entity {
        let entity = PointSpawner::new(
            &meshes_materials.meshes,
            &meshes_materials.materials.respawn_points,
            &OutlineSettings::default(),
            Self {
                id: id as u16,
                ..default()
            },
        )
        .pos(pos)
        .spawn_command(commands);
        add_respawn_point_preview(
            entity,
            commands,
            &meshes_materials.meshes,
            &meshes_materials.materials.respawn_points,
        );
        entity
    }
}
impl CannonPoint {
    pub fn spawn(commands: &mut Commands, meshes_materials: &KmpMeshesMaterials, pos: Vec3, id: usize) -> Entity {
        PointSpawner::new(
            &meshes_materials.meshes,
            &meshes_materials.materials.cannon_points,
            &OutlineSettings::default(),
            Self {
                id: id as u16,
                ..default()
            },
        )
        .pos(pos)
        .spawn_command(commands)
    }
}
impl BattleFinishPoint {
    pub fn spawn(commands: &mut Commands, meshes_materials: &KmpMeshesMaterials, pos: Vec3, id: usize) -> Entity {
        PointSpawner::new(
            &meshes_materials.meshes,
            &meshes_materials.materials.battle_finish_points,
            &OutlineSettings::default(),
            Self { id: id as u16 },
        )
        .pos(pos)
        .spawn_command(commands)
    }
}

impl ItemPathPoint {
    pub fn spawn(
        commands: &mut Commands,
        meshes_materials: &KmpMeshesMaterials,
        pos: Vec3,
        prev_nodes: HashSet<Entity>,
    ) -> Entity {
        let entity = PathPointSpawner::<_, ItemPathMarker>::new(
            &meshes_materials.meshes,
            &meshes_materials.materials.item_paths,
            &OutlineSettings::default(),
            Self::default(),
        )
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

impl EnemyPathPoint {
    pub fn spawn(
        commands: &mut Commands,
        meshes_materials: &KmpMeshesMaterials,
        pos: Vec3,
        prev_nodes: HashSet<Entity>,
    ) -> Entity {
        let entity = PathPointSpawner::<_, EnemyPathMarker>::new(
            &meshes_materials.meshes,
            &meshes_materials.materials.enemy_paths,
            &OutlineSettings::default(),
            Self::default(),
        )
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
