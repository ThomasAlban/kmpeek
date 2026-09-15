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
#[derive(Resource, Default, Serialize, Deserialize, PartialEq, Clone)]
pub struct TrackInfo {
    pub track_type: TrackType,
    pub lap_count: u8,
    pub speed_mod: f32,
    /// STGI bytes 8..10 have no verified interpretation; retain the full word.
    #[serde(default)]
    pub padding_1: u16,
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
    /// Unknown trailing KTPT word, not an editor ordering index.
    #[serde(default)]
    pub padding: u16,
}
impl Default for StartPoint {
    fn default() -> Self {
        Self {
            player_index: -1,
            padding: 0,
        }
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
impl CheckpointKind {
    pub fn from_cp_type(cp_type: i8) -> Option<Self> {
        match cp_type {
            -1 => Some(Self::Normal),
            0 => Some(Self::LapCount),
            id @ 1..=127 => Some(Self::Key(id as u8)),
            _ => None,
        }
    }

    pub fn cp_type(&self) -> i8 {
        match self {
            Self::Normal => -1,
            Self::LapCount => 0,
            Self::Key(id) => *id as i8,
        }
    }
}

#[derive(Component, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct CheckpointMarker;

// --- OBJECT COMPONENTS ---
#[derive(Component, Default, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Object {
    pub object_id: u16,
    /// GOBJ's second word participates in extended presence flags. Zeroing it
    /// during conversion can change valid custom objects even without an edit.
    #[serde(default)]
    pub padding: u16,
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
    /// Opaque AREA trailing word, independent of the type-specific settings.
    #[serde(default)]
    pub padding: u16,
}
impl Default for AreaPoint {
    fn default() -> Self {
        Self {
            shape: AreaShape::default(),
            kind: AreaKind::default(),
            priority: 0,
            scale: vec3(10000., 10000., 10000.),
            show_area: false,
            padding: 0,
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
    pub duration: f32,
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
    /// Stored local ID, separate from OrderId and checkpoint entity links.
    /// Kept internally for patch preservation; rebuild assigns it automatically.
    #[serde(default)]
    pub respawn_id: u16,
    /// Full signed JGPT payload; dividing by 100 discarded valid source data.
    pub extra_data: i16,
}

// --- CANNON POINT COMPONENTS
#[derive(Component, Default, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CannonPoint {
    /// Stored CNPT ID; canonical rebuild owns dense renumbering, not conversion.
    #[serde(default)]
    pub id: u16,
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
pub struct BattleFinishPoint {
    /// Stored MSPT ID, intentionally renumbered only by canonical rebuild.
    #[serde(default)]
    pub id: u16,
    /// Preserve this word without claiming an unverified gameplay meaning.
    #[serde(default)]
    pub unknown: u16,
}

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
            speed_mod: data.speed_mod(),
            padding_1: data.padding_1,
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
            padding_1: self.padding_1,
            padding_2: Stgi::encode_speed_mod(self.speed_mod),
        }
    }
}
impl KmpComponent for StartPoint {
    type KmpFormat = Ktpt;
    fn from_kmp(data: &Ktpt, _: &mut World) -> Self {
        Self {
            player_index: data.player_index,
            padding: data.padding,
        }
    }
    fn to_kmp(&self, transform: Transform, _: &mut World, _: Entity) -> Ktpt {
        Ktpt {
            position: transform.translation.into(),
            rotation: get_euler_rot(&transform).into(),
            player_index: self.player_index,
            padding: self.padding,
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
            bullet_cant_drop: data.setting_2 & 0x1 != 0,
            low_shell_priority: data.setting_2 & 0x2 != 0,
        }
    }
    fn to_kmp(&self, transform: Transform, _: &mut World, _: Entity) -> Itpt {
        Itpt {
            position: transform.translation.into(),
            bullet_control: self.bullet_control,
            setting_1: match self.bullet_height {
                ItemPathBulletHeight::IgnorePointHeight => 0,
                ItemPathBulletHeight::Auto => 1,
                ItemPathBulletHeight::FollowPointHeight => 2,
                ItemPathBulletHeight::MushroomPads => 3,
            },
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
            kind: CheckpointKind::from_cp_type(data.cp_type).unwrap_or_else(|| {
                world.resource_mut::<KmpErrors>().add("Invalid CKPT setting found");
                CheckpointKind::Normal
            }),
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
            cp_type: self.kind.cp_type(),
            respawn_pos: {
                const FALLBACK_RESPAWN_ID: u8 = 0;
                let maybe_respawn_e = world.entity(e).get::<CheckpointRespawnLink>();
                if let Some(respawn_e) = maybe_respawn_e {
                    let respawn_entity_id_map = world.resource::<KmpSectionEntityIdMap<RespawnPoint>>();
                    let maybe_respawn_id = respawn_entity_id_map.get(&**respawn_e).copied();
                    maybe_respawn_id
                        .and_then(|id| u8::try_from(id).ok())
                        .unwrap_or(FALLBACK_RESPAWN_ID)
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
            padding: data.padding,
            scale: data.scale.into(),
            settings: data.settings,
            presence: data.presence_flags,
        }
    }
    fn to_kmp(&self, transform: Transform, world: &mut World, e: Entity) -> Gobj {
        Gobj {
            object_id: self.object_id,
            padding: self.padding,
            position: transform.translation.into(),
            rotation: get_euler_rot(&transform).into(),
            scale: self.scale.into(),
            route: {
                let maybe_route = world.entity(e).get::<RouteLink>();
                if let Some(route) = maybe_route {
                    let id = world.resource::<KmpSectionEntityIdMap<RouteSettings>>().get(&**route);
                    if let Some(id) = id {
                        *id
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

        // Travel along the route, stopping if malformed data contains a cycle
        // or a stale entity reference.
        let mut cur_e = e;
        let mut visited = EntityHashSet::from_iter([e]);
        while let Some(next_e) = world
            .get::<KmpPathNode>(cur_e)
            .and_then(|path| path.next_nodes.iter().next())
            .copied()
        {
            if !visited.insert(next_e) {
                break;
            }
            let Ok((route_point, transform)) = q.get(world, next_e) else {
                break;
            };
            points.push((route_point.clone(), *transform, next_e));
            cur_e = next_e;
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
                    group_id: data.setting_1,
                },
                10 => AreaKind::FallBoundary,
                _ => {
                    world.resource_mut::<KmpErrors>().add("Invalid AREA type found");
                    AreaKind::default()
                }
            },
            show_area: false,
            padding: data.padding,
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
                    id.copied().unwrap_or(0xffff)
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
            padding: self.padding,
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
            duration: data.duration,
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
                id.copied().unwrap_or(0xff) as u8
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
            duration: self.duration,
        }
    }
}
// Local IDs are internal file data, not editor OrderIds or user settings. Keep
// conversions inverse for patch preservation; canonical rebuild renumbers them.
impl KmpComponent for RespawnPoint {
    type KmpFormat = Jgpt;
    fn from_kmp(data: &Jgpt, _: &mut World) -> Self {
        Self {
            respawn_id: data.respawn_id,
            extra_data: data.extra_data,
        }
    }
    fn to_kmp(&self, transform: Transform, _: &mut World, _: Entity) -> Jgpt {
        Jgpt {
            position: transform.translation.into(),
            rotation: get_euler_rot(&transform).into(),
            respawn_id: self.respawn_id,
            extra_data: self.extra_data,
        }
    }
}
impl KmpComponent for CannonPoint {
    type KmpFormat = Cnpt;
    fn from_kmp(data: &Cnpt, world: &mut World) -> Self {
        Self {
            id: data.id,
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
            id: self.id,
            shoot_effect: self.shoot_effect as i16,
        }
    }
}
impl KmpComponent for BattleFinishPoint {
    type KmpFormat = Mspt;
    fn from_kmp(data: &Mspt, _: &mut World) -> Self {
        Self {
            id: data.id,
            unknown: data.unknown,
        }
    }
    fn to_kmp(&self, transform: Transform, _: &mut World, _: Entity) -> Mspt {
        Mspt {
            position: transform.translation.into(),
            rotation: get_euler_rot(&transform).into(),
            id: self.id,
            unknown: self.unknown,
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
        commands.queue(|world: &mut World| {
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

#[cfg(test)]
mod conversion_tests {
    use super::*;

    #[test]
    fn checkpoint_type_raw_values_round_trip() {
        for cp_type in -1..=127 {
            let kind = CheckpointKind::from_cp_type(cp_type).unwrap();
            assert_eq!(kind.cp_type(), cp_type);
        }
        assert!(CheckpointKind::from_cp_type(-2).is_none());
        assert!(CheckpointKind::from_cp_type(i8::MIN).is_none());
    }

    #[test]
    fn respawn_extra_data_retains_full_signed_word() {
        let mut world = World::new();
        let entity = world.spawn(OrderId(123)).id();
        // Non-multiples of 100, negatives and extremes used to be quantized/wrapped.
        for extra_data in [i16::MIN, -101, -1, 0, 1, 199, 32767] {
            let raw = Jgpt {
                extra_data,
                ..default()
            };
            let point = RespawnPoint::from_kmp(&raw, &mut world);
            let encoded = point.to_kmp(Transform::IDENTITY, &mut world, entity);
            assert_eq!(encoded.extra_data, extra_data);
            assert_eq!(encoded.respawn_id, raw.respawn_id);
        }
    }

    /// Stored IDs are internal file bookkeeping, independent of editor order.
    #[test]
    fn cannon_and_finish_ids_preserve_raw_values() {
        let mut world = World::new();
        let entity = world.spawn(OrderId(65536)).id();
        let cannon = CannonPoint { id: 4321, ..default() };
        let finish = BattleFinishPoint {
            id: 1234,
            unknown: 0xabcd,
        };
        assert_eq!(cannon.to_kmp(Transform::IDENTITY, &mut world, entity).id, 4321);
        let encoded = finish.to_kmp(Transform::IDENTITY, &mut world, entity);
        assert_eq!(encoded.id, 1234);
        assert_eq!(encoded.unknown, 0xabcd);
    }

    /// Newly exposed opaque words must survive component conversion in both modes;
    /// they are actual stored data, not scratch padding for the editor to zero.
    #[test]
    fn raw_record_words_survive_editor_conversion() {
        let mut world = World::new();
        world.init_resource::<KmpErrors>();
        let raw = Ktpt {
            padding: 0x1234,
            ..default()
        };
        assert_eq!(
            StartPoint::from_kmp(&raw, &mut world)
                .to_kmp(Transform::IDENTITY, &mut world, Entity::PLACEHOLDER)
                .padding,
            0x1234
        );
        let raw = Gobj {
            padding: 0x4321,
            ..default()
        };
        let entity = world.spawn_empty().id();
        assert_eq!(
            Object::from_kmp(&raw, &mut world)
                .to_kmp(Transform::IDENTITY, &mut world, entity)
                .padding,
            0x4321
        );
        let raw = Area {
            padding: 0x5678,
            ..default()
        };
        assert_eq!(
            AreaPoint::from_kmp(&raw, &mut world)
                .to_kmp(Transform::IDENTITY, &mut world, entity)
                .padding,
            0x5678
        );
        let raw = Stgi {
            padding_1: 0x8765,
            ..default()
        };
        assert_eq!(
            TrackInfo::from_kmp(&raw, &mut world)
                .to_kmp(Transform::IDENTITY, &mut world, entity)
                .padding_1,
            0x8765
        );
    }

    #[test]
    fn track_speed_uses_high_float_word_without_normalizing_zero() {
        let mut world = World::new();
        for (bits, value) in [(0, 0.), (0x3f00, 0.5), (0x3f80, 1.), (0x3fc0, 1.5), (0x4000, 2.)] {
            let raw = Stgi {
                padding_2: bits,
                ..default()
            };
            let track = TrackInfo::from_kmp(&raw, &mut world);
            assert_eq!(track.speed_mod, value);
            assert_eq!(
                track
                    .to_kmp(Transform::IDENTITY, &mut world, Entity::PLACEHOLDER)
                    .padding_2,
                bits
            );
        }
        assert_eq!(Stgi::encode_speed_mod(f32::from_bits(0x3f81abcd)), 0x3f81);
    }

    #[test]
    fn item_path_height_encoding_matches_kmp_values() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        for (setting_1, bullet_height) in [
            (0, ItemPathBulletHeight::IgnorePointHeight),
            (1, ItemPathBulletHeight::Auto),
            (2, ItemPathBulletHeight::FollowPointHeight),
            (3, ItemPathBulletHeight::MushroomPads),
        ] {
            let raw = Itpt {
                setting_1,
                bullet_control: 42.,
                ..Default::default()
            };
            let point = ItemPathPoint::from_kmp(&raw, &mut world);
            assert_eq!(point.bullet_height, bullet_height);
            let encoded = point.to_kmp(Transform::IDENTITY, &mut world, entity);
            assert_eq!(encoded.setting_1, setting_1);
            assert_eq!(encoded.bullet_control, raw.bullet_control);
        }
    }

    #[test]
    fn item_path_flags_decode_independently_of_height_and_unknown_bits() {
        let mut world = World::new();
        world.init_resource::<KmpErrors>();
        let entity = world.spawn_empty().id();
        // Height 5 used to incorrectly enable bullet_cant_drop.
        for setting_1 in [0, 1, 2, 3, 5] {
            for unknown_bits in [0, 4, 0x8000, 0xfffc] {
                for flags in 0..=3 {
                    let raw = Itpt {
                        setting_1,
                        setting_2: unknown_bits | flags,
                        ..Default::default()
                    };
                    let point = ItemPathPoint::from_kmp(&raw, &mut world);
                    assert_eq!(point.bullet_cant_drop, flags & 1 != 0);
                    assert_eq!(point.low_shell_priority, flags & 2 != 0);
                    let encoded = point.to_kmp(Transform::IDENTITY, &mut world, entity);
                    // Unsupported bits belong to the original/baseline preservation layer.
                    assert_eq!(encoded.setting_2, flags);
                }
            }
        }
    }

    #[test]
    fn object_unload_group_uses_setting_one_in_both_directions() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        for group_id in [0, 1, 255, u16::MAX] {
            let raw = Area {
                kind: 9,
                setting_1: group_id,
                setting_2: group_id ^ u16::MAX,
                ..Default::default()
            };
            let point = AreaPoint::from_kmp(&raw, &mut world);
            assert_eq!(point.kind, AreaKind::ObjectUnload { group_id });
            let encoded = point.to_kmp(Transform::IDENTITY, &mut world, entity);
            assert_eq!(encoded.kind, 9);
            assert_eq!(encoded.setting_1, group_id);
            assert_eq!(encoded.setting_2, 0);
            assert_eq!(AreaPoint::from_kmp(&encoded, &mut world).kind, point.kind);
        }
    }
}
