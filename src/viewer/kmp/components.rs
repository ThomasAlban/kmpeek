#![allow(dead_code)]

use super::{
    checkpoints::{checkpoint_spawner, CheckpointHeight, CheckpointLeft, CheckpointRespawnLink},
    ordering::OrderId,
    path::{spawn_path, KmpPathNode},
    point::spawn_point,
    routes::RouteLink,
    Ckpt, Cnpt, Jgpt, KmpErrors, KmpSectionName, Mspt,
};
use crate::{
    ui::util::{get_euler_rot, set_euler_rot},
    util::kmp_file::{Area, Came, Enpt, Gobj, Itpt, Ktpt, Poti, PotiPoint, Stgi},
    viewer::kmp::KmpSectionEntityIdMap,
};
use bevy::{ecs::entity::EntityHashSet, math::vec3, prelude::*};
use binrw::{BinRead, BinWrite};
use bon::builder;
use derive_new::new;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Component, Default, Clone, Copy, new)]
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
#[derive(Resource, Component, Default, Serialize, Deserialize, PartialEq, Clone)]
pub struct TrackInfo {
    pub track_type: TrackType,
    pub lap_count: u8,
    pub speed_mod: f32,
    pub lens_flare_color: [u8; 4],
    pub lens_flare_flashing: bool,
    pub first_player_pos: FirstPlayerPos,
    pub narrow_player_spacing: bool,
}
#[derive(Default, Display, EnumIter, EnumString, IntoStaticStr, PartialEq, Clone, Serialize, Deserialize)]
pub enum TrackType {
    #[default]
    Race,
    Battle,
}
#[derive(Default, Display, EnumIter, EnumString, IntoStaticStr, PartialEq, Clone, Serialize, Deserialize)]
pub enum FirstPlayerPos {
    #[default]
    Left,
    Right,
}

// --- START POINT COMPONENTS ---
#[derive(Component, Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct StartPoint {
    pub player_index: i16,
}
impl Default for StartPoint {
    fn default() -> Self {
        Self { player_index: -1 }
    }
}

// --- ENEMY PATH COMPONENTS ---
#[derive(Component, Clone, Copy, PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct EnemyPathPoint {
    pub leniency: f32,
    pub setting_1: EnemyPathSetting1,
    pub setting_2: EnemyPathSetting2,
    pub setting_3: u8,
}
#[derive(
    Display, EnumString, IntoStaticStr, EnumIter, Default, PartialEq, Clone, Copy, Debug, Serialize, Deserialize,
)]
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
#[derive(
    Display, EnumString, IntoStaticStr, EnumIter, Default, PartialEq, Clone, Copy, Debug, Serialize, Deserialize,
)]
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
#[derive(Component, PartialEq, Clone, Default, Debug, Serialize, Deserialize)]
pub struct ItemPathPoint {
    pub bullet_control: f32,
    pub bullet_height: ItemPathBulletHeight,
    pub bullet_cant_drop: bool,
    pub low_shell_priority: bool,
}

#[derive(
    Display, EnumString, IntoStaticStr, EnumIter, Default, PartialEq, Clone, Copy, Debug, Serialize, Deserialize,
)]
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
#[derive(Component, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Checkpoint {
    pub kind: CheckpointKind,
    // will contain link to respawn entity
}

#[derive(
    Component, PartialEq, Clone, Default, Debug, Display, EnumString, IntoStaticStr, EnumIter, Serialize, Deserialize,
)]
pub enum CheckpointKind {
    #[default]
    Normal,
    Key(u8),
    #[strum(serialize = "Lap Count")]
    LapCount,
}

#[derive(Component, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct CheckpointMarker;

// --- OBJECT COMPONENTS ---
#[derive(Component, Default, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Object {
    pub object_id: u16,
    pub scale: Vec3,
    pub settings: [u16; 8],
    pub presence: u16,
}

// --- ROUTE COMPONENTS ---
#[derive(Component, Default, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct RouteSettings {
    pub smooth_motion: bool,
    pub loop_style: RouteLoopStyle,
}

#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum RouteLoopStyle {
    #[default]
    Cyclic,
    Mirror,
}

#[derive(Component, Default, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct RoutePoint {
    pub settings: u16,
    pub additional_settings: u16,
}

// --- AREA COMPONENTS ---
#[derive(Component, Clone, PartialEq, Serialize, Deserialize, Debug)]
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
#[derive(
    Display, EnumString, IntoStaticStr, EnumIter, Default, Clone, Copy, PartialEq, Serialize, Deserialize, Debug,
)]
pub enum AreaShape {
    #[default]
    Box,
    Cylinder,
}
#[derive(Display, EnumString, IntoStaticStr, EnumIter, Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum AreaKind {
    Camera {
        cam_index: u8,
    },
    #[strum(serialize = "Env Effect")]
    EnvEffect(AreaEnvEffectObject),
    #[strum(serialize = "Fog Effect")]
    FogEffect {
        bfg_entry: u16,
        setting_2: u16,
    },
    #[strum(serialize = "Moving Road")]
    /// Important: This variant has a route associated with it
    MovingRoad,
    #[strum(serialize = "Force Recalc")]
    ForceRecalc {
        enemy_path_id: u8,
    },
    #[strum(serialize = "Minimap Control")]
    MinimapControl {
        setting_1: u16,
        setting_2: u16,
    },
    #[strum(serialize = "Bloom Effect")]
    BloomEffect {
        bblm_file: u16,
        fade_time: u16,
    },
    #[strum(serialize = "Enable Boos")]
    EnableBoos,
    #[strum(serialize = "Object Group")]
    ObjectGroup {
        group_id: u16,
    },
    #[strum(serialize = "Object Unload")]
    ObjectUnload {
        group_id: u16,
    },
    #[strum(serialize = "Fall Boundary")]
    FallBoundary,
}
impl Default for AreaKind {
    fn default() -> Self {
        Self::Camera { cam_index: 0 }
    }
}
#[derive(
    Default, Clone, Copy, PartialEq, Display, EnumString, IntoStaticStr, EnumIter, Serialize, Deserialize, Debug,
)]
pub enum AreaEnvEffectObject {
    #[default]
    EnvKareha,
    EnvKarehaUp,
}

// --- CAMERA COMPONENTS ---
#[derive(Component, Default, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct KmpCamera {
    pub kind: KmpCameraKind,
    pub next_index: u8,
    pub shake: u8,
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
#[derive(
    Default, Clone, Copy, PartialEq, Display, EnumString, IntoStaticStr, EnumIter, Serialize, Deserialize, Debug,
)]
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

#[derive(Component)]
pub struct KmpCameraIntroStart;

// --- RESPAWN POINT COMPONENTS ---
#[derive(Component, Default, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct RespawnPoint {
    pub sound_trigger: i8,
}

// --- CANNON POINT COMPONENTS
#[derive(Component, Default, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CannonPoint {
    pub shoot_effect: CannonShootEffect,
}
#[derive(
    Default, Display, EnumIter, EnumString, IntoStaticStr, PartialEq, Clone, Copy, Serialize, Deserialize, Debug,
)]
pub enum CannonShootEffect {
    #[default]
    Straight,
    Curved,
    #[strum(serialize = "Curved & Slow")]
    CurvedSlow,
}

#[derive(Component, Default, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct BattleFinishPoint;

//
// --- CONVERT COMPONENTS FROM KMP STORAGE FORMAT ---
//

pub trait KmpComponent
where
    Self: Component + Clone,
{
    type KmpFormat: 'static
        + for<'a> BinRead<Args<'a> = ()>
        + for<'a> BinWrite<Args<'a> = ()>
        + KmpSectionName
        + Clone
        + Default;

    fn from_kmp(data: &Self::KmpFormat, world: &mut World) -> Self;
    fn to_kmp(&self, transform: Transform, world: &mut World, self_e: Entity) -> Self::KmpFormat;
}

impl KmpComponent for TrackInfo {
    type KmpFormat = Stgi;
    fn from_kmp(data: &Stgi, world: &mut World) -> Self {
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
                    world
                        .resource_mut::<KmpErrors>()
                        .add("Invalid STGI First Player Pos found");
                    FirstPlayerPos::default()
                }
            },
            narrow_player_spacing: data.driver_distance == 1,
        }
    }
    fn to_kmp(&self, _: Transform, _: &mut World, _: Entity) -> Stgi {
        Stgi {
            lap_count: self.lap_count,
            flare_color: self.lens_flare_color,
            lens_flare_flashing: self.lens_flare_flashing as u8,
            pole_pos: match self.first_player_pos {
                FirstPlayerPos::Left => 0,
                FirstPlayerPos::Right => 1,
            },
            driver_distance: self.narrow_player_spacing as u8,
            padding_1: 0,
            padding_2: 0,
        }
    }
}
impl KmpComponent for StartPoint {
    type KmpFormat = Ktpt;
    fn from_kmp(data: &Ktpt, _: &mut World) -> Self {
        Self {
            player_index: data.player_index,
        }
    }
    fn to_kmp(&self, transform: Transform, _: &mut World, _: Entity) -> Ktpt {
        Ktpt {
            position: transform.translation.into(),
            rotation: get_euler_rot(&transform).into(),
            player_index: self.player_index,
        }
    }
}
impl KmpComponent for EnemyPathPoint {
    type KmpFormat = Enpt;
    fn from_kmp(data: &Enpt, world: &mut World) -> Self {
        Self {
            leniency: data.leniency,
            setting_1: match data.setting_1 {
                0 => EnemyPathSetting1::None,
                1 => EnemyPathSetting1::RequiresMushroom,
                2 => EnemyPathSetting1::UseMushroom,
                3 => EnemyPathSetting1::Wheelie,
                4 => EnemyPathSetting1::EndWheelie,
                _ => {
                    world.resource_mut::<KmpErrors>().add("Invalid ENPT setting 1 found");
                    EnemyPathSetting1::default()
                }
            },
            setting_2: match data.setting_2 {
                0 => EnemyPathSetting2::None,
                1 => EnemyPathSetting2::EndDrift,
                2 => EnemyPathSetting2::ForbidDrift,
                3 => EnemyPathSetting2::ForceDrift,
                _ => {
                    world.resource_mut::<KmpErrors>().add("Invalid ENPT setting 2 found");
                    EnemyPathSetting2::default()
                }
            },
            setting_3: data.setting_3,
        }
    }
    fn to_kmp(&self, transform: Transform, _: &mut World, _: Entity) -> Enpt {
        Enpt {
            position: transform.translation.into(),
            leniency: self.leniency,
            setting_1: self.setting_1 as u16,
            setting_2: self.setting_2 as u8,
            setting_3: self.setting_3,
        }
    }
}
impl KmpComponent for ItemPathPoint {
    type KmpFormat = Itpt;
    fn from_kmp(data: &Itpt, world: &mut World) -> Self {
        Self {
            bullet_control: data.bullet_control,
            bullet_height: match data.setting_1 {
                0 => ItemPathBulletHeight::IgnorePointHeight,
                1 => ItemPathBulletHeight::Auto,
                2 => ItemPathBulletHeight::FollowPointHeight,
                3 => ItemPathBulletHeight::MushroomPads,
                _ => {
                    world.resource_mut::<KmpErrors>().add("Invalid ITPT setting 1 found");
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
    fn to_kmp(&self, transform: Transform, _: &mut World, _: Entity) -> Itpt {
        Itpt {
            position: transform.translation.into(),
            bullet_control: self.bullet_control,
            setting_1: self.bullet_height as u16,
            setting_2: match (self.bullet_cant_drop, self.low_shell_priority) {
                (true, true) => 3,
                (true, false) => 1,
                (false, true) => 2,
                (false, false) => 0,
            },
        }
    }
}
impl KmpComponent for Checkpoint {
    type KmpFormat = Ckpt;
    fn from_kmp(data: &Ckpt, world: &mut World) -> Self {
        Self {
            kind: match data.cp_type {
                -1 => CheckpointKind::Normal,
                0 => CheckpointKind::LapCount,
                id @ 1..=127 => CheckpointKind::Key(id as u8),
                _ => {
                    world.resource_mut::<KmpErrors>().add("Invalid CKPT setting found");
                    CheckpointKind::Normal
                }
            },
        }
    }
    fn to_kmp(&self, transform: Transform, world: &mut World, e: Entity) -> Ckpt {
        Ckpt {
            cp_left: transform.translation.xz().into(),
            cp_right: world
                .entity(world.entity(e).get::<CheckpointLeft>().unwrap().right)
                .get::<Transform>()
                .unwrap()
                .translation
                .xz()
                .into(),
            cp_type: match self.kind {
                CheckpointKind::Normal => -1,
                CheckpointKind::LapCount => 0,
                CheckpointKind::Key(id) => id as i8,
            },
            respawn_pos: {
                const FALLBACK_RESPAWN_ID: u8 = 0;
                let maybe_respawn_e = world.entity(e).get::<CheckpointRespawnLink>();
                if let Some(respawn_e) = maybe_respawn_e {
                    let respawn_entity_id_map = world.resource::<KmpSectionEntityIdMap<RespawnPoint>>();
                    let maybe_respawn_id = respawn_entity_id_map.get(&**respawn_e).copied();
                    maybe_respawn_id.unwrap_or(FALLBACK_RESPAWN_ID)
                } else {
                    FALLBACK_RESPAWN_ID
                }
            },
            prev_cp: {
                let kmp_path = world.entity(e).get::<KmpPathNode>().unwrap();
                (|| {
                    // check that there is only 1 prev node
                    (kmp_path.prev_nodes.len() == 1).then_some(())?;
                    let prev_node = world.entity(*kmp_path.prev_nodes.iter().next()?);

                    // check that the prev node has only 1 next node
                    (prev_node.get::<KmpPathNode>()?.next_nodes.len() == 1).then_some(())?;
                    // check we are not the overall start because if we are, then we are the start of a group
                    (world.entity(e).get::<PathOverallStart>().is_none()).then_some(())?;

                    Some(**prev_node.get::<OrderId>().unwrap() as u8)
                })()
                .unwrap_or(0xff)
            },
            next_cp: {
                let kmp_path = world.entity(e).get::<KmpPathNode>().unwrap();
                (|| {
                    // check that there is only 1 next node
                    (kmp_path.next_nodes.len() == 1).then_some(())?;
                    let next_node = world.entity(*kmp_path.next_nodes.iter().next()?);

                    // check that the next node has only 1 prex node
                    (next_node.get::<KmpPathNode>()?.prev_nodes.len() == 1).then_some(())?;
                    // check that the next node is not the overall start because if it is, we are the end of a group
                    (next_node.get::<PathOverallStart>().is_none()).then_some(())?;

                    Some(**next_node.get::<OrderId>().unwrap() as u8)
                })()
                .unwrap_or(0xff)
            },
        }
    }
}
impl KmpComponent for Object {
    type KmpFormat = Gobj;
    fn from_kmp(data: &Gobj, _: &mut World) -> Self {
        Self {
            object_id: data.object_id,
            scale: data.scale.into(),
            settings: data.settings,
            presence: data.presence_flags,
        }
    }
    fn to_kmp(&self, transform: Transform, world: &mut World, e: Entity) -> Gobj {
        Gobj {
            object_id: self.object_id,
            padding: 0,
            position: transform.translation.into(),
            rotation: get_euler_rot(&transform).into(),
            scale: self.scale.into(),
            route: {
                let maybe_route = world.entity(e).get::<RouteLink>();
                if let Some(route) = maybe_route {
                    let id = world.resource::<KmpSectionEntityIdMap<RouteSettings>>().get(&**route);
                    if let Some(id) = id {
                        *id as u16
                    } else {
                        0xffff
                    }
                } else {
                    0xffff
                }
            },
            settings: self.settings,
            presence_flags: self.presence,
        }
    }
}
impl KmpComponent for RouteSettings {
    type KmpFormat = Poti;
    fn from_kmp(data: &Poti, world: &mut World) -> Self {
        Self {
            smooth_motion: match data.setting_1 {
                0 => false,
                1 => true,
                _ => {
                    world.resource_mut::<KmpErrors>().add("Invalid Route setting found");
                    false
                }
            },
            loop_style: match data.setting_2 {
                0 => RouteLoopStyle::Cyclic,
                1 => RouteLoopStyle::Mirror,
                _ => {
                    world.resource_mut::<KmpErrors>().add("Invalid Route setting found");
                    RouteLoopStyle::Cyclic
                }
            },
        }
    }
    fn to_kmp(&self, transform: Transform, world: &mut World, e: Entity) -> Poti {
        // start off with a vec containing the route pt, transform and entity of the first entity in the route
        let mut points = vec![(world.entity(e).get::<RoutePoint>().unwrap().clone(), transform, e)];

        let mut q = world.query::<(&RoutePoint, &Transform)>();

        //  travel along the route, pushing each route point to 'points' as we go
        let mut cur_e = e;
        while let Some(e) = world
            .entity(cur_e)
            .get::<KmpPathNode>()
            .and_then(|x| x.next_nodes.iter().next())
            .copied()
        {
            let data = q.get(world, e).unwrap();
            points.push((data.0.clone(), *data.1, e));
            cur_e = e;
        }
        // convert each route point to storage format
        let points: Vec<PotiPoint> = points
            .into_iter()
            .map(|(route_pt, transform, e)| route_pt.to_kmp(transform, world, e))
            .collect();

        Poti {
            num_points: points.len() as u16,
            setting_1: if self.smooth_motion { 1 } else { 0 },
            setting_2: match self.loop_style {
                RouteLoopStyle::Cyclic => 0,
                RouteLoopStyle::Mirror => 1,
            },
            points,
        }
    }
}
impl KmpComponent for RoutePoint {
    type KmpFormat = PotiPoint;
    fn from_kmp(data: &PotiPoint, _: &mut World) -> Self {
        Self {
            settings: data.setting_1,
            additional_settings: data.setting_2,
        }
    }
    fn to_kmp(&self, transform: Transform, _: &mut World, _: Entity) -> PotiPoint {
        PotiPoint {
            position: transform.translation.into(),
            setting_1: self.settings,
            setting_2: self.additional_settings,
        }
    }
}
impl KmpComponent for AreaPoint {
    type KmpFormat = Area;
    fn from_kmp(data: &Area, world: &mut World) -> Self {
        Self {
            shape: match data.shape {
                0 => AreaShape::Box,
                1 => AreaShape::Cylinder,
                _ => {
                    world.resource_mut::<KmpErrors>().add("Invalid AREA shape found");
                    AreaShape::Box
                }
            },
            priority: data.priority,
            scale: Vec3::from(data.scale) * vec3(5000., 10000., 5000.),
            kind: match data.kind {
                0 => AreaKind::Camera {
                    cam_index: data.came_index,
                },
                1 => AreaKind::EnvEffect(match data.setting_1 {
                    0 => AreaEnvEffectObject::EnvKareha,
                    1 => AreaEnvEffectObject::EnvKarehaUp,
                    _ => {
                        world
                            .resource_mut::<KmpErrors>()
                            .add("Invalid AREA env effect object found");
                        AreaEnvEffectObject::EnvKareha
                    }
                }),
                2 => AreaKind::FogEffect {
                    bfg_entry: data.setting_1,
                    setting_2: data.setting_2,
                },
                3 => AreaKind::MovingRoad,
                4 => AreaKind::ForceRecalc {
                    enemy_path_id: data.enpt_id,
                },
                5 => AreaKind::MinimapControl {
                    setting_1: data.setting_1,
                    setting_2: data.setting_2,
                },
                6 => AreaKind::BloomEffect {
                    bblm_file: data.setting_1,
                    fade_time: data.setting_2,
                },
                7 => AreaKind::EnableBoos,
                8 => AreaKind::ObjectGroup {
                    group_id: data.setting_1,
                },
                9 => AreaKind::ObjectUnload {
                    group_id: data.setting_2,
                },
                10 => AreaKind::FallBoundary,
                _ => {
                    world.resource_mut::<KmpErrors>().add("Invalid AREA type found");
                    AreaKind::default()
                }
            },
            show_area: false,
        }
    }
    fn to_kmp(&self, transform: Transform, world: &mut World, e: Entity) -> Area {
        let mut area_came_index = None;
        let mut area_route = None;
        let mut area_setting_1 = None;
        let mut area_setting_2 = None;
        let mut area_enpt_id = None;
        let kind: u8 = match self.kind {
            AreaKind::Camera { cam_index } => {
                area_came_index = Some(cam_index);
                0
            }
            AreaKind::EnvEffect(env_eff_obj) => {
                area_setting_1 = Some(env_eff_obj as u16);
                1
            }
            AreaKind::FogEffect { bfg_entry, setting_2 } => {
                area_setting_1 = Some(bfg_entry);
                area_setting_2 = Some(setting_2);
                2
            }
            AreaKind::MovingRoad => {
                let route_id = if let Some(route) = world.entity(e).get::<RouteLink>() {
                    let id = world.resource::<KmpSectionEntityIdMap<RouteSettings>>().get(&**route);
                    id.map(|x| *x as u16).unwrap_or(0xffff)
                } else {
                    0xffff
                };
                area_route = Some(route_id as u8);
                3
            }
            AreaKind::ForceRecalc { enemy_path_id } => {
                area_enpt_id = Some(enemy_path_id);
                4
            }
            AreaKind::MinimapControl { setting_1, setting_2 } => {
                area_setting_1 = Some(setting_1);
                area_setting_2 = Some(setting_2);
                5
            }
            AreaKind::BloomEffect { bblm_file, fade_time } => {
                area_setting_1 = Some(bblm_file);
                area_setting_2 = Some(fade_time);
                6
            }
            AreaKind::EnableBoos => 7,
            AreaKind::ObjectGroup { group_id } => {
                area_setting_1 = Some(group_id);
                8
            }
            AreaKind::ObjectUnload { group_id } => {
                area_setting_1 = Some(group_id);
                9
            }
            AreaKind::FallBoundary => 10,
        };
        let came_index = area_came_index.unwrap_or(0xff);
        let route = area_route.unwrap_or(0);
        let enpt_id = area_enpt_id.unwrap_or(0);
        let setting_1 = area_setting_1.unwrap_or(0);
        let setting_2 = area_setting_2.unwrap_or(0);
        Area {
            position: transform.translation.into(),
            rotation: get_euler_rot(&transform).into(),
            shape: self.shape as u8,
            priority: self.priority,
            scale: (self.scale / vec3(5000., 10000., 5000.)).into(),
            kind,
            came_index,
            setting_1,
            setting_2,
            route,
            enpt_id,
        }
    }
}
impl KmpComponent for KmpCamera {
    type KmpFormat = Came;
    fn from_kmp(data: &Came, world: &mut World) -> Self {
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
                    world.resource_mut::<KmpErrors>().add("Invalid CAME type found");
                    KmpCameraKind::Goal
                }
            },
            next_index: data.next_index,
            shake: data.shake,
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
    fn to_kmp(&self, transform: Transform, world: &mut World, e: Entity) -> Came {
        Came {
            position: transform.translation.into(),
            rotation: get_euler_rot(&transform).into(),
            kind: self.kind as u8,
            next_index: self.next_index,
            shake: self.shake,
            route: if let Some(route) = world.entity(e).get::<RouteLink>() {
                let id = world.resource::<KmpSectionEntityIdMap<RouteSettings>>().get(&**route);
                id.copied().unwrap_or(0xff)
            } else {
                0xff
            },
            point_velocity: self.point_velocity,
            zoom_velocity: self.zoom_velocity,
            view_velocity: self.view_velocity,
            start: self.start,
            movie: self.movie,
            zoom_start: self.zoom_start,
            zoom_end: self.zoom_end,
            view_start: self.view_start.into(),
            view_end: self.view_end.into(),
            time: self.time,
        }
    }
}
impl KmpComponent for RespawnPoint {
    type KmpFormat = Jgpt;
    fn from_kmp(data: &Jgpt, _: &mut World) -> Self {
        Self {
            sound_trigger: if data.extra_data >= 0 {
                ((data.extra_data / 100) - 1) as i8
            } else {
                -1
            },
        }
    }
    fn to_kmp(&self, transform: Transform, world: &mut World, e: Entity) -> Jgpt {
        Jgpt {
            position: transform.translation.into(),
            rotation: get_euler_rot(&transform).into(),
            respawn_id: **world.entity(e).get::<OrderId>().unwrap() as u16,
            extra_data: ((self.sound_trigger as i16 + 1) * 100),
        }
    }
}
impl KmpComponent for CannonPoint {
    type KmpFormat = Cnpt;
    fn from_kmp(data: &Cnpt, world: &mut World) -> Self {
        Self {
            shoot_effect: match data.shoot_effect {
                0 => CannonShootEffect::Straight,
                1 => CannonShootEffect::Curved,
                2 => CannonShootEffect::CurvedSlow,
                _ => {
                    world.resource_mut::<KmpErrors>().add("Invalid CNPT type found");
                    CannonShootEffect::Straight
                }
            },
        }
    }
    fn to_kmp(&self, transform: Transform, _: &mut World, _: Entity) -> Cnpt {
        Cnpt {
            position: transform.translation.into(),
            rotation: get_euler_rot(&transform).into(),
            shoot_effect: self.shoot_effect as i16,
        }
    }
}
impl KmpComponent for BattleFinishPoint {
    type KmpFormat = Mspt;
    fn from_kmp(_: &Mspt, _: &mut World) -> Self {
        Self
    }
    fn to_kmp(&self, transform: Transform, _: &mut World, _: Entity) -> Mspt {
        Mspt {
            position: transform.translation.into(),
            rotation: get_euler_rot(&transform).into(),
            unknown: 0,
        }
    }
}

//
// --- IMPLEMENT HOW TO SPAWN EACH COMPONENT ---
//

pub trait Spawn
where
    Self: Component + Sized + Clone + Default,
{
    fn spawn(spawner: Spawner<Self>, world: &mut World) -> Entity;
}

macro_rules! impl_spawn_point {
    ($ty:ty) => {
        impl Spawn for $ty {
            fn spawn(spawner: Spawner<Self>, world: &mut World) -> Entity {
                spawn_point(spawner, world)
            }
        }
    };
}
macro_rules! impl_spawn_path {
    ($ty:ty) => {
        impl Spawn for $ty {
            fn spawn(spawner: Spawner<Self>, world: &mut World) -> Entity {
                spawn_path(spawner, world)
            }
        }
    };
}

impl_spawn_point!(StartPoint);
impl_spawn_path!(EnemyPathPoint);
impl_spawn_path!(ItemPathPoint);
impl_spawn_point!(Object);
impl_spawn_path!(RoutePoint);
impl_spawn_point!(AreaPoint);
impl_spawn_point!(KmpCamera);
impl_spawn_point!(RespawnPoint);
impl_spawn_point!(CannonPoint);
impl_spawn_point!(BattleFinishPoint);

impl Spawn for Checkpoint {
    fn spawn(spawner: Spawner<Self>, world: &mut World) -> Entity {
        let pos = spawner.pos.xz();
        let (left, right) = checkpoint_spawner()
            .cp(spawner.component)
            .pos((pos, pos))
            .visible(spawner.visible)
            .maybe_right_e(spawner.e)
            .height(world.resource::<CheckpointHeight>().0)
            .maybe_order_id(spawner.order_id)
            .world(world)
            .call();

        if let Some(prev_nodes) = spawner.prev_nodes {
            for prev_left in prev_nodes {
                KmpPathNode::link_nodes(prev_left, left, world);
                let prev_right = world.entity(prev_left).get::<CheckpointLeft>().unwrap().right;
                KmpPathNode::link_nodes(prev_right, right, world);
            }
        }
        right
    }
}

#[builder]
pub struct Spawner<T: Component + Spawn + Clone + Default> {
    #[builder(default)]
    pub pos: Vec3,
    #[builder(default)]
    pub rot: Vec3,
    #[builder(default)]
    pub component: T,
    pub prev_nodes: Option<EntityHashSet>,
    #[builder(default = 6)]
    pub max: u8,
    pub order_id: Option<u32>,
    pub e: Option<Entity>,
    #[builder(default = true)]
    pub visible: bool,
    pub route: Option<Entity>,
}
impl<T: Component + Spawn + Clone + Default> Spawner<T> {
    pub fn get_transform(&self) -> Transform {
        let mut t = Transform::from_translation(self.pos);
        set_euler_rot(self.rot, &mut t);
        t
    }
    pub fn spawn_command(mut self, commands: &mut Commands) -> Entity {
        let e = self.e.unwrap_or_else(|| commands.spawn_empty().id());
        self.e = Some(e);
        commands.add(|world: &mut World| {
            self.spawn(world);
        });
        e
    }
    pub fn spawn(self, world: &mut World) -> Entity {
        T::spawn(self, world)
    }
}

//
// --- MAX CONNECTED PATHS ---
//

pub trait MaxConnectedPath {
    const MAX_CONNECTED: u8;
}
impl MaxConnectedPath for EnemyPathPoint {
    const MAX_CONNECTED: u8 = 6;
}
impl MaxConnectedPath for ItemPathPoint {
    const MAX_CONNECTED: u8 = 6;
}
impl MaxConnectedPath for Checkpoint {
    const MAX_CONNECTED: u8 = 6;
}
impl MaxConnectedPath for RoutePoint {
    const MAX_CONNECTED: u8 = 1;
}
