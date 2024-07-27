#![allow(dead_code)]

use super::{
    checkpoints::{CheckpointHeight, CheckpointLeft, CheckpointSpawner},
    ordering::OrderId,
    path::{spawn_path, KmpPathNode},
    point::spawn_point,
    Ckpt, Cnpt, Jgpt, KmpSectionName, Mspt, Section, SectionHeader,
};
use crate::{
    ui::util::quat_to_euler,
    util::kmp_file::{Area, Came, Enpt, Gobj, Itpt, Ktpt, Poti, PotiPoint, Stgi},
};
use bevy::{
    ecs::{entity::EntityHashSet, world::Command},
    math::vec3,
    prelude::*,
    utils::HashSet,
};
use binrw::{BinRead, BinWrite};
use serde::{Deserialize, Serialize};
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
    Key,
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
#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum AreaShape {
    #[default]
    Box,
    Cylinder,
}
#[derive(Display, EnumString, IntoStaticStr, EnumIter, Clone, PartialEq, Serialize, Deserialize, Debug)]
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
    MovingRoad {
        route_id: u16,
    },
    #[strum(serialize = "Force Recalc")]
    ForceRecalc,
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
#[derive(Default, Clone, PartialEq, Display, EnumString, IntoStaticStr, EnumIter, Serialize, Deserialize, Debug)]
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
#[derive(Default, Clone, PartialEq, Display, EnumString, IntoStaticStr, EnumIter, Serialize, Deserialize, Debug)]
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
#[derive(Component, Default, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct RespawnPoint {
    pub sound_trigger: i8,
}

// --- CANNON POINT COMPONENTS
#[derive(Component, Default, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CannonPoint {
    pub shoot_effect: CannonShootEffect,
}
#[derive(Default, Display, EnumIter, EnumString, IntoStaticStr, PartialEq, Clone, Serialize, Deserialize, Debug)]
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
    fn from_kmp(data: &T, errors: &mut Vec<KmpError>) -> Self;
}

impl FromKmp<Stgi> for TrackInfo {
    fn from_kmp(data: &Stgi, errors: &mut Vec<KmpError>) -> Self {
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
    fn from_kmp(data: &Ktpt, _: &mut Vec<KmpError>) -> Self {
        Self {
            player_index: data.player_index,
        }
    }
}
impl FromKmp<Enpt> for EnemyPathPoint {
    fn from_kmp(data: &Enpt, errors: &mut Vec<KmpError>) -> Self {
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
        }
    }
}
impl FromKmp<Itpt> for ItemPathPoint {
    fn from_kmp(data: &Itpt, errors: &mut Vec<KmpError>) -> Self {
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
        }
    }
}
impl FromKmp<Ckpt> for Checkpoint {
    fn from_kmp(data: &Ckpt, errors: &mut Vec<KmpError>) -> Self {
        Self {
            kind: match data.cp_type {
                -1 => CheckpointKind::Normal,
                0 => CheckpointKind::LapCount,
                1..=127 => CheckpointKind::Key,
                _ => {
                    errors.push(KmpError::new("Invalid CKPT setting found"));
                    CheckpointKind::Normal
                }
            },
        }
    }
}
impl FromKmp<Gobj> for Object {
    fn from_kmp(data: &Gobj, _: &mut Vec<KmpError>) -> Self {
        Self {
            object_id: data.object_id,
            scale: data.scale.into(),
            settings: data.settings,
            presence: data.presence_flags,
        }
    }
}
impl FromKmp<Poti> for RouteSettings {
    fn from_kmp(data: &Poti, errors: &mut Vec<KmpError>) -> Self {
        Self {
            smooth_motion: match data.setting_1 {
                0 => false,
                1 => true,
                _ => {
                    errors.push(KmpError::new("Invalid Route setting found"));
                    false
                }
            },
            loop_style: match data.setting_2 {
                0 => RouteLoopStyle::Cyclic,
                1 => RouteLoopStyle::Mirror,
                _ => {
                    errors.push(KmpError::new("Invalid Route setting found"));
                    RouteLoopStyle::Cyclic
                }
            },
        }
    }
}
impl FromKmp<PotiPoint> for RoutePoint {
    fn from_kmp(data: &PotiPoint, _: &mut Vec<KmpError>) -> Self {
        Self {
            settings: data.setting_1,
            additional_settings: data.setting_2,
        }
    }
}
impl FromKmp<Area> for AreaPoint {
    fn from_kmp(data: &Area, errors: &mut Vec<KmpError>) -> Self {
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
                0 => AreaKind::Camera {
                    cam_index: data.came_index,
                },
                1 => AreaKind::EnvEffect(match data.setting_1 {
                    0 => AreaEnvEffectObject::EnvKareha,
                    1 => AreaEnvEffectObject::EnvKarehaUp,
                    _ => {
                        errors.push(KmpError::new("Invalid AREA env effect object found"));
                        AreaEnvEffectObject::EnvKareha
                    }
                }),
                2 => AreaKind::FogEffect {
                    bfg_entry: data.setting_1,
                    setting_2: data.setting_2,
                },
                3 => AreaKind::MovingRoad {
                    route_id: data.enpt_id.into(),
                },
                4 => AreaKind::ForceRecalc,
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
                    errors.push(KmpError::new("Invalid AREA type found"));
                    AreaKind::default()
                }
            },
            show_area: false,
        }
    }
}
impl FromKmp<Came> for KmpCamera {
    fn from_kmp(data: &Came, errors: &mut Vec<KmpError>) -> Self {
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
    fn from_kmp(data: &Jgpt, _: &mut Vec<KmpError>) -> Self {
        Self {
            sound_trigger: if data.extra_data >= 0 {
                ((data.extra_data / 100) - 1) as i8
            } else {
                -1
            },
        }
    }
}
impl FromKmp<Cnpt> for CannonPoint {
    fn from_kmp(data: &Cnpt, errors: &mut Vec<KmpError>) -> Self {
        Self {
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
    fn from_kmp(_: &Mspt, _: &mut Vec<KmpError>) -> Self {
        Self
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
        let pos = spawner.transform.translation.xz();
        let mut cp_spawner = CheckpointSpawner::new(spawner.component)
            .pos(pos, pos)
            .visible(spawner.visible)
            .height(world.resource::<CheckpointHeight>().0);
        if let Some(order_id) = spawner.order_id {
            cp_spawner = cp_spawner.order_id(order_id);
        }
        let (left, right) = cp_spawner.spawn(world);

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

pub struct Spawner<T: Component + Spawn + Clone + Default> {
    pub transform: Transform,
    pub component: T,
    pub prev_nodes: Option<HashSet<Entity>>,
    pub max: u8,
    pub order_id: Option<u32>,
    pub e: Option<Entity>,
    pub visible: bool,
    pub route: Option<Entity>,
}
impl<T: Component + Spawn + Clone + Default> Default for Spawner<T> {
    fn default() -> Self {
        Self {
            transform: Transform::default(),
            component: T::default(),
            prev_nodes: None,
            max: 6,
            order_id: None,
            e: None,
            visible: true,
            route: None,
        }
    }
}

impl<T: Component + Spawn + Clone + Default> Spawner<T> {
    pub fn new(component: T) -> Self {
        Self { component, ..default() }
    }
    pub fn transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }
    pub fn pos(mut self, pos: impl Into<Vec3>) -> Self {
        self.transform.translation = pos.into();
        self
    }
    pub fn rot(mut self, rot: Quat) -> Self {
        self.transform.rotation = rot;
        self
    }
    pub fn prev_nodes(mut self, prev_nodes: impl IntoIterator<Item = Entity>) -> Self {
        self.prev_nodes = Some(prev_nodes.into_iter().collect());
        self
    }
    pub fn max_connected(mut self, max: impl Into<u8>) -> Self {
        self.max = max.into();
        self
    }
    pub fn order_id(mut self, id: u32) -> Self {
        self.order_id = Some(id);
        self
    }
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn entity(mut self, entity: Entity) -> Self {
        self.e = Some(entity);
        self
    }
    pub fn route(mut self, route: Entity) -> Self {
        self.route = Some(route);
        self
    }
    pub fn maybe_route(mut self, route: Option<Entity>) -> Self {
        self.route = route;
        self
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

//
// --- GETTING BACK TO STORAGE FORMAT FROM WORLD ---
//

trait ToKmp<T> {
    fn to_kmp(&self, pos: Vec3, rot: Vec3) -> T;
}

impl ToKmp<Ktpt> for StartPoint {
    fn to_kmp(&self, pos: Vec3, rot: Vec3) -> Ktpt {
        Ktpt {
            position: pos.into(),
            rotation: rot.into(),
            player_index: self.player_index,
        }
    }
}

impl ToKmp<Cnpt> for CannonPoint {
    fn to_kmp(&self, pos: Vec3, rot: Vec3) -> Cnpt {
        Cnpt {
            position: pos.into(),
            rotation: rot.into(),
            shoot_effect: self.shoot_effect.clone() as i16,
        }
    }
}

impl ToKmp<Mspt> for BattleFinishPoint {
    fn to_kmp(&self, pos: Vec3, rot: Vec3) -> Mspt {
        Mspt {
            position: pos.into(),
            rotation: rot.into(),
            unknown: 0,
        }
    }
}

trait GetKmpSection<T> {
    fn get_kmp_section(world: &mut World) -> Self;
}

impl<C, T> GetKmpSection<C> for Section<T>
where
    C: ToKmp<T> + Component,
    T: KmpSectionName,
    for<'a> T: BinRead<Args<'a> = ()> + 'a,
    for<'a> T: BinWrite<Args<'a> = ()> + 'a,
{
    fn get_kmp_section(world: &mut World) -> Self {
        let mut q = world.query::<(&C, &Transform)>();

        let mut section = Section {
            section_header: SectionHeader {
                section_name: T::SECTION_NAME,
                num_entries: 0,
                additional_value: 0,
            },
            entries: Vec::new(),
        };

        let items: Vec<_> = q.iter(world).sort::<&OrderId>().collect();
        let num_entries = items.len();
        let mut kmp_items = Vec::with_capacity(num_entries);
        for (item, transform) in items {
            let rot = quat_to_euler(transform);
            kmp_items.push(item.to_kmp(transform.translation, rot))
        }

        section.entries = kmp_items;
        section.section_header.num_entries = num_entries as u16;

        section
    }
}
