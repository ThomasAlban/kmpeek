use crate::util::kmp_file::*;
use bevy::prelude::*;
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Component, Default)]
pub struct KmpSection;

pub trait FromKmp<T: KmpData> {
    fn from_kmp(kmp_data: T) -> Self;
}

// components attached to kmp entities, to store data about them:

// --- START POINT COMPONENTS ---
#[derive(Component, Default)]
pub struct StartPoint {
    pub player_index: i16,
}
impl FromKmp<Ktpt> for StartPoint {
    fn from_kmp(kmp_data: Ktpt) -> Self {
        Self {
            player_index: kmp_data.player_index,
        }
    }
}

// --- ENEMY PATH COMPONENTS ---
#[derive(Component, Default)]
pub struct EnemyPathMarker;
#[derive(Component, Clone)]
pub struct EnemyPathPoint {
    pub leniency: f32,
    pub setting_1: u16,
    pub setting_2: u8,
    pub setting_3: u8,
}
impl FromKmp<Enpt> for EnemyPathPoint {
    fn from_kmp(kmp_data: Enpt) -> Self {
        Self {
            leniency: kmp_data.leniency,
            setting_1: kmp_data.setting_1,
            setting_2: kmp_data.setting_2,
            setting_3: kmp_data.setting_3,
        }
    }
}

// --- ITEM PATH COMPONENTS ---
#[derive(Component, Default)]
pub struct ItemPathMarker;
#[derive(Component)]
pub struct ItemPathPoint {
    pub bullet_bill_control: f32,
    pub setting_1: u16,
    pub setting_2: u16,
}
impl FromKmp<Itpt> for ItemPathPoint {
    fn from_kmp(kmp_data: Itpt) -> Self {
        Self {
            bullet_bill_control: kmp_data.bullet_bill_control,
            setting_1: kmp_data.setting_1,
            setting_2: kmp_data.setting_2,
        }
    }
}

// --- OBJECT COMPONENTS ---
#[derive(Component)]
pub struct Object {
    pub object_id: u16,
    pub scale: Vec3,
    pub route: u16,
    pub settings: [u16; 8],
    pub presence_flags: u16,
}
impl FromKmp<Gobj> for Object {
    fn from_kmp(kmp_data: Gobj) -> Self {
        Self {
            object_id: kmp_data.object_id,
            scale: kmp_data.scale,
            route: kmp_data.route,
            settings: kmp_data.settings,
            presence_flags: kmp_data.presence_flags,
        }
    }
}

// --- ROUTE COMPONENTS ---
#[derive(Component)]
pub struct Route {
    pub setting_1: u8,
    pub setting_2: u8,
}
impl FromKmp<Poti> for Route {
    fn from_kmp(kmp_data: Poti) -> Self {
        Self {
            setting_1: kmp_data.setting_1,
            setting_2: kmp_data.setting_2,
        }
    }
}
#[derive(Component, Default)]
pub struct RouteMarker;
#[derive(Component)]
pub struct RoutePoint {
    pub setting_1: u16,
    pub setting_2: u16,
}
impl FromKmp<PotiPoint> for RoutePoint {
    fn from_kmp(kmp_data: PotiPoint) -> Self {
        Self {
            setting_1: kmp_data.setting_1,
            setting_2: kmp_data.setting_2,
        }
    }
}

// --- AREA COMPONENTS ---
#[derive(Component)]
pub struct AreaPoint {
    pub shape: AreaShape,
    pub kind: AreaKind,
    pub priority: u8,
    pub scale: Vec3,
}
impl FromKmp<Area> for AreaPoint {
    fn from_kmp(kmp_data: Area) -> Self {
        Self {
            shape: kmp_data.shape.into(),
            priority: kmp_data.priority,
            scale: kmp_data.scale,
            kind: match kmp_data.kind {
                0 => AreaKind::Camera(AreaCameraIndex(kmp_data.came_index)),
                1 => AreaKind::EnvEffect(kmp_data.setting_1.into()),
                2 => AreaKind::FogEffect(
                    AreaBfgEntry(kmp_data.setting_1),
                    AreaSetting2(kmp_data.setting_2),
                ),
                3 => AreaKind::MovingRoad(AreaEnemyPointId(kmp_data.enpt_id)),
                4 => AreaKind::ForceRecalc,
                5 => AreaKind::MinimapControl(
                    AreaSetting1(kmp_data.setting_1),
                    AreaSetting2(kmp_data.setting_2),
                ),
                6 => AreaKind::BloomEffect(
                    AreaBblmFile(kmp_data.setting_1),
                    AreaFadeTime(kmp_data.setting_2),
                ),
                7 => AreaKind::EnableBoos,
                8 => AreaKind::ObjectGroup(AreaGroupId(kmp_data.setting_1)),
                9 => AreaKind::ObjectUnload(AreaGroupId(kmp_data.setting_1)),
                10 => AreaKind::FallBoundary,
                _ => {
                    warn!("Invalid AREA type found, which has been set to Camera");
                    AreaKind::default()
                }
            },
        }
    }
}

#[derive(Display, EnumString, IntoStaticStr, EnumIter)]
pub enum AreaShape {
    Box,
    Cylinder,
}
impl From<u8> for AreaShape {
    fn from(value: u8) -> Self {
        match value {
            0 => AreaShape::Box,
            1 => AreaShape::Cylinder,
            _ => {
                warn!("Invalid AREA shape found, which has been set to Box");
                AreaShape::Box
            }
        }
    }
}
#[derive(Display, EnumString, IntoStaticStr, EnumIter)]
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
#[derive(Default)]
pub struct AreaCameraIndex(pub u8);
#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default)]
pub enum AreaEnvEffectObject {
    #[default]
    EnvKareha,
    EnvKarehaUp,
}
impl From<u16> for AreaEnvEffectObject {
    fn from(value: u16) -> Self {
        match value {
            0 => AreaEnvEffectObject::EnvKareha,
            1 => AreaEnvEffectObject::EnvKarehaUp,
            _ => {
                warn!("Invalid AREA env effect object found, which has been set to EnvKareha");
                AreaEnvEffectObject::EnvKareha
            }
        }
    }
}
#[derive(Default)]
pub struct AreaBfgEntry(pub u16);
#[derive(Default)]
pub struct AreaSetting1(pub u16);
#[derive(Default)]
pub struct AreaSetting2(pub u16);
#[derive(Default)]
pub struct AreaRouteId(pub u8);
#[derive(Default)]
pub struct AreaEnemyPointId(pub u8);
#[derive(Default)]
pub struct AreaBblmFile(pub u16);
#[derive(Default)]
pub struct AreaFadeTime(pub u16);
#[derive(Default)]
pub struct AreaGroupId(pub u16);

// --- CAMERA COMPONENTS ---
#[derive(Component)]
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
impl FromKmp<Came> for KmpCamera {
    fn from_kmp(kmp_data: Came) -> Self {
        Self {
            kind: kmp_data.kind.into(),
            next_index: kmp_data.next_index,
            shake: kmp_data.shake,
            route: kmp_data.route,
            point_velocity: kmp_data.point_velocity,
            zoom_velocity: kmp_data.zoom_velocity,
            view_velocity: kmp_data.view_velocity,
            start: kmp_data.start,
            movie: kmp_data.movie,
            zoom_start: kmp_data.zoom_start,
            zoom_end: kmp_data.zoom_end,
            view_start: kmp_data.view_start,
            view_end: kmp_data.view_end,
            time: kmp_data.time,
        }
    }
}
pub enum KmpCameraKind {
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
impl From<u8> for KmpCameraKind {
    fn from(value: u8) -> Self {
        use KmpCameraKind::*;
        match value {
            0 => Goal,
            1 => FixSearch,
            2 => PathSearch,
            3 => KartFollow,
            4 => KartPathFollow,
            5 => OP_FixMoveAt,
            6 => OP_PathMoveAt,
            7 => MiniGame,
            8 => MissionSuccess,
            9 => Unknown,
            _ => {
                warn!("Invalid CAME type found, which has been set to Goal");
                Goal
            }
        }
    }
}

#[derive(Component)]
pub struct RespawnPoint;
#[derive(Component)]
pub struct CannonPoint;
#[derive(Component)]
pub struct FinishPoint;
