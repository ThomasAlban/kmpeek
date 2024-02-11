use crate::util::kmp_file::{Area, Came, Enpt, Gobj, Itpt, Ktpt, Poti, PotiPoint, Stgi};
use bevy::prelude::*;
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

use super::{meshes_materials::KmpMeshesMaterials, point::PointSpawner, settings::OutlineSettings, Cnpt, Jgpt, Mspt};

#[derive(Component, Default)]
pub struct KmpSelectablePoint;

// components attached to kmp entities, to store data about them:

// --- GENERAL PATH COMPONENTS ---
#[derive(Component, Default)]
pub struct PathStart;
#[derive(Component, Default)]
pub struct PathOverallStart;

// --- TRACK INFO COMPONENTS ---
#[derive(Component, Default)]
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
#[derive(Component, Clone, Copy, PartialEq)]
pub struct EnemyPathPoint {
    pub leniency: f32,
    pub setting_1: EnemyPathSetting1,
    pub setting_2: EnemyPathSetting2,
    pub setting_3: u8,
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
#[derive(Component, PartialEq, Clone)]
pub struct ItemPathPoint {
    pub bullet_control: f32,
    pub bullet_height: ItemPathBulletHeight,
    pub bullet_cant_drop: bool,
    pub low_shell_priority: bool,
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

// --- OBJECT COMPONENTS ---
#[derive(Component, Default, Clone)]
pub struct Object {
    pub object_id: u16,
    pub scale: Vec3,
    pub route: u16,
    pub settings: [u16; 8],
    pub presence_flags: u16,
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
#[derive(Component, Default, Clone)]
pub struct AreaPoint {
    pub shape: AreaShape,
    pub kind: AreaKind,
    pub priority: u8,
    pub scale: Vec3,
}
#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default, Clone)]
pub enum AreaShape {
    #[default]
    Box,
    Cylinder,
}
#[derive(Display, EnumString, IntoStaticStr, EnumIter, Clone)]
pub enum AreaKind {
    Camera(AreaCameraIndex),
    #[strum(serialize = "Env Effect")]
    EnvEffect(AreaEnvEffectObject),
    #[strum(serialize = "Fog Effect")]
    FogEffect(AreaBfgEntry, AreaSetting2),
    #[strum(serialize = "Moving Road")]
    MovingRoad(AreaEnemyPointId),
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
#[derive(Default, Clone)]
pub struct AreaCameraIndex(pub u8);
#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default, Clone)]
pub enum AreaEnvEffectObject {
    #[default]
    EnvKareha,
    EnvKarehaUp,
}
#[derive(Default, Clone)]
pub struct AreaBfgEntry(pub u16);
#[derive(Default, Clone)]
pub struct AreaSetting1(pub u16);
#[derive(Default, Clone)]
pub struct AreaSetting2(pub u16);
#[derive(Default, Clone)]
pub struct AreaRouteId(pub u8);
#[derive(Default, Clone)]
pub struct AreaEnemyPointId(pub u8);
#[derive(Default, Clone)]
pub struct AreaBblmFile(pub u16);
#[derive(Default, Clone)]
pub struct AreaFadeTime(pub u16);
#[derive(Default, Clone)]
pub struct AreaGroupId(pub u16);

// --- CAMERA COMPONENTS ---
#[derive(Component, Default, Clone)]
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
#[derive(Default, Clone)]
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

#[derive(Component, Default, Clone)]
pub struct RespawnPoint;
#[derive(Component, Default, Clone)]
pub struct CannonPoint;
#[derive(Component, Default, Clone)]
pub struct BattleFinishPoint;

//
// --- CONVERT COMPONENTS FROM KMP STORAGE FORMAT ---
//

pub trait FromKmp<T> {
    fn from_kmp(data: &T) -> Self;
}

impl FromKmp<Stgi> for TrackInfo {
    fn from_kmp(data: &Stgi) -> Self {
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
                    warn!("Invalid STGI First Player Pos found");
                    FirstPlayerPos::default()
                }
            },
            narrow_player_spacing: data.driver_distance == 1,
        }
    }
}
impl FromKmp<Ktpt> for StartPoint {
    fn from_kmp(data: &Ktpt) -> Self {
        Self {
            player_index: data.player_index,
        }
    }
}
impl FromKmp<Enpt> for EnemyPathPoint {
    fn from_kmp(data: &Enpt) -> Self {
        Self {
            leniency: data.leniency,
            setting_1: match data.setting_1 {
                0 => EnemyPathSetting1::None,
                1 => EnemyPathSetting1::RequiresMushroom,
                2 => EnemyPathSetting1::UseMushroom,
                3 => EnemyPathSetting1::Wheelie,
                4 => EnemyPathSetting1::EndWheelie,
                _ => {
                    warn!("Invalid ENPT setting 1 found");
                    EnemyPathSetting1::default()
                }
            },
            setting_2: match data.setting_2 {
                0 => EnemyPathSetting2::None,
                1 => EnemyPathSetting2::EndDrift,
                2 => EnemyPathSetting2::ForbidDrift,
                3 => EnemyPathSetting2::ForceDrift,
                _ => {
                    warn!("Invalid ENPT setting 2 found");
                    EnemyPathSetting2::default()
                }
            },
            setting_3: data.setting_3,
        }
    }
}
impl FromKmp<Itpt> for ItemPathPoint {
    fn from_kmp(data: &Itpt) -> Self {
        Self {
            bullet_control: data.bullet_control,
            bullet_height: match data.setting_1 {
                0 => ItemPathBulletHeight::IgnorePointHeight,
                1 => ItemPathBulletHeight::Auto,
                2 => ItemPathBulletHeight::FollowPointHeight,
                3 => ItemPathBulletHeight::MushroomPads,
                _ => {
                    warn!("Invalid ITPT setting 1 found");
                    ItemPathBulletHeight::default()
                }
            },
            bullet_cant_drop: data.setting_2 == 1 || data.setting_2 == 3 || data.setting_1 == 5 || data.setting_2 == 7,
            low_shell_priority: data.setting_2 == 2
                || data.setting_2 == 3
                || data.setting_2 == 6
                || data.setting_2 == 7,
        }
    }
}
impl FromKmp<Gobj> for Object {
    fn from_kmp(data: &Gobj) -> Self {
        Self {
            object_id: data.object_id,
            scale: data.scale.into(),
            route: data.route,
            settings: data.settings,
            presence_flags: data.presence_flags,
        }
    }
}
impl FromKmp<Poti> for Route {
    fn from_kmp(data: &Poti) -> Self {
        Self {
            setting_1: data.setting_1,
            setting_2: data.setting_2,
        }
    }
}
impl FromKmp<PotiPoint> for RoutePoint {
    fn from_kmp(data: &PotiPoint) -> Self {
        Self {
            setting_1: data.setting_1,
            setting_2: data.setting_2,
        }
    }
}
impl FromKmp<Area> for AreaPoint {
    fn from_kmp(data: &Area) -> Self {
        Self {
            shape: match data.shape {
                0 => AreaShape::Box,
                1 => AreaShape::Cylinder,
                _ => {
                    warn!("Invalid AREA shape found");
                    AreaShape::Box
                }
            },
            priority: data.priority,
            scale: data.scale.into(),
            kind: match data.kind {
                0 => AreaKind::Camera(AreaCameraIndex(data.came_index)),
                1 => AreaKind::EnvEffect(match data.setting_1 {
                    0 => AreaEnvEffectObject::EnvKareha,
                    1 => AreaEnvEffectObject::EnvKarehaUp,
                    _ => {
                        warn!("Invalid AREA env effect object found");
                        AreaEnvEffectObject::EnvKareha
                    }
                }),
                2 => AreaKind::FogEffect(AreaBfgEntry(data.setting_1), AreaSetting2(data.setting_2)),
                3 => AreaKind::MovingRoad(AreaEnemyPointId(data.enpt_id)),
                4 => AreaKind::ForceRecalc,
                5 => AreaKind::MinimapControl(AreaSetting1(data.setting_1), AreaSetting2(data.setting_2)),
                6 => AreaKind::BloomEffect(AreaBblmFile(data.setting_1), AreaFadeTime(data.setting_2)),
                7 => AreaKind::EnableBoos,
                8 => AreaKind::ObjectGroup(AreaGroupId(data.setting_1)),
                9 => AreaKind::ObjectUnload(AreaGroupId(data.setting_1)),
                10 => AreaKind::FallBoundary,
                _ => {
                    warn!("Invalid AREA type found, which has been set to Camera");
                    AreaKind::default()
                }
            },
        }
    }
}
impl FromKmp<Came> for KmpCamera {
    fn from_kmp(data: &Came) -> Self {
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
                    warn!("Invalid CAME type found, which has been set to Goal");
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
    fn from_kmp(_data: &Jgpt) -> Self {
        Self
    }
}
impl FromKmp<Cnpt> for CannonPoint {
    fn from_kmp(_data: &Cnpt) -> Self {
        Self
    }
}
impl FromKmp<Mspt> for BattleFinishPoint {
    fn from_kmp(_data: &Mspt) -> Self {
        Self
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
impl_spawnable_point!(CannonPoint, cannon_points);
impl_spawnable_point!(BattleFinishPoint, battle_finish_points);
