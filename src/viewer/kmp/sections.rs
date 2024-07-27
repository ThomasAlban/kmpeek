use std::marker::PhantomData;

use bevy::{
    ecs::system::{Resource, SystemParam},
    prelude::*,
};
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

use super::{
    AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, EnemyPathPoint, ItemPathPoint, KmpCamera, Object,
    RespawnPoint, RoutePoint, StartPoint, TrackInfo,
};

pub fn section_plugin(app: &mut App) {
    app.add_event::<KmpEditModeChange>();
}

#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum KmpSection {
    #[strum(serialize = "Start Points")]
    StartPoints,
    #[strum(serialize = "Enemy Paths")]
    EnemyPaths,
    #[strum(serialize = "Item Paths")]
    ItemPaths,
    Checkpoints,
    #[strum(serialize = "Respawn Points")]
    RespawnPoints,
    Objects,
    Routes,
    Areas,
    Cameras,
    #[strum(serialize = "Cannon Points")]
    CannonPoints,
    #[strum(serialize = "Battle Finish Points")]
    BattleFinishPoints,
    #[default]
    #[strum(serialize = "Track Info")]
    TrackInfo,
}
#[derive(Resource)]
pub struct KmpEditMode<T: Component>(PhantomData<T>);
impl<T: Component + ToKmpSection> Default for KmpEditMode<T> {
    fn default() -> Self {
        KmpEditMode(PhantomData)
    }
}

#[derive(Event, Default)]
pub struct KmpEditModeChange;

pub fn change_kmp_edit_mode<T: Component + ToKmpSection>(world: &mut World) {
    world.remove_resource::<KmpEditMode<StartPoint>>();
    world.remove_resource::<KmpEditMode<EnemyPathPoint>>();
    world.remove_resource::<KmpEditMode<ItemPathPoint>>();
    world.remove_resource::<KmpEditMode<Checkpoint>>();
    world.remove_resource::<KmpEditMode<RespawnPoint>>();
    world.remove_resource::<KmpEditMode<Object>>();
    world.remove_resource::<KmpEditMode<RoutePoint>>();
    world.remove_resource::<KmpEditMode<AreaPoint>>();
    world.remove_resource::<KmpEditMode<KmpCamera>>();
    world.remove_resource::<KmpEditMode<CannonPoint>>();
    world.remove_resource::<KmpEditMode<BattleFinishPoint>>();
    world.remove_resource::<KmpEditMode<TrackInfo>>();

    world.init_resource::<KmpEditMode<T>>();
    world.send_event_default::<KmpEditModeChange>();
}

pub fn get_kmp_section(world: &mut World) -> KmpSection {
    if world.contains_resource::<KmpEditMode<StartPoint>>() {
        KmpSection::StartPoints
    } else if world.contains_resource::<KmpEditMode<EnemyPathPoint>>() {
        KmpSection::EnemyPaths
    } else if world.contains_resource::<KmpEditMode<ItemPathPoint>>() {
        KmpSection::ItemPaths
    } else if world.contains_resource::<KmpEditMode<Checkpoint>>() {
        KmpSection::Checkpoints
    } else if world.contains_resource::<KmpEditMode<RespawnPoint>>() {
        KmpSection::RespawnPoints
    } else if world.contains_resource::<KmpEditMode<Object>>() {
        KmpSection::Objects
    } else if world.contains_resource::<KmpEditMode<RoutePoint>>() {
        KmpSection::Routes
    } else if world.contains_resource::<KmpEditMode<AreaPoint>>() {
        KmpSection::Areas
    } else if world.contains_resource::<KmpEditMode<KmpCamera>>() {
        KmpSection::Cameras
    } else if world.contains_resource::<KmpEditMode<CannonPoint>>() {
        KmpSection::CannonPoints
    } else if world.contains_resource::<KmpEditMode<BattleFinishPoint>>() {
        KmpSection::BattleFinishPoints
    } else {
        KmpSection::TrackInfo
    }
}

macro_rules! add_for_all_components {
    (@system $app:expr, $sys:ident) => {
        $app.add_systems(
            Update,
            (
                $sys::<StartPoint>,
                $sys::<EnemyPathPoint>,
                $sys::<ItemPathPoint>,
                $sys::<Checkpoint>,
                $sys::<RespawnPoint>,
                $sys::<Object>,
                $sys::<RoutePoint>,
                $sys::<AreaPoint>,
                $sys::<KmpCamera>,
                $sys::<CannonPoint>,
                $sys::<BattleFinishPoint>,
            ),
        )
    };
    (@event $app:expr, $ev:ident) => {
        $app.add_event::<$ev<StartPoint>>()
            .add_event::<$ev<EnemyPathPoint>>()
            .add_event::<$ev<ItemPathPoint>>()
            .add_event::<$ev<Checkpoint>>()
            .add_event::<$ev<RespawnPoint>>()
            .add_event::<$ev<Object>>()
            .add_event::<$ev<RoutePoint>>()
            .add_event::<$ev<AreaPoint>>()
            .add_event::<$ev<KmpCamera>>()
            .add_event::<$ev<CannonPoint>>()
            .add_event::<$ev<BattleFinishPoint>>()
    };
    (@plugin $app:expr, $plugin:ident) => {
        $app.add_plugins((
            $plugin::<StartPoint>,
            $plugin::<EnemyPathPoint>,
            $plugin::<ItemPathPoint>,
            $plugin::<Checkpoint>,
            $plugin::<RespawnPoint>,
            $plugin::<Object>,
            $plugin::<RoutePoint>,
            $plugin::<AreaPoint>,
            $plugin::<KmpCamera>,
            $plugin::<CannonPoint>,
            $plugin::<BattleFinishPoint>,
        ))
    };
}
pub(crate) use add_for_all_components;

pub trait ToKmpSection {
    fn to_kmp_section() -> KmpSection;
}
macro_rules! impl_to_kmp_sect {
    ($comp:ty, $sect:expr) => {
        impl ToKmpSection for $comp {
            fn to_kmp_section() -> KmpSection {
                $sect
            }
        }
    };
}

impl_to_kmp_sect!(StartPoint, KmpSection::StartPoints);
impl_to_kmp_sect!(EnemyPathPoint, KmpSection::EnemyPaths);
impl_to_kmp_sect!(ItemPathPoint, KmpSection::ItemPaths);
impl_to_kmp_sect!(Checkpoint, KmpSection::Checkpoints);
impl_to_kmp_sect!(RespawnPoint, KmpSection::RespawnPoints);
impl_to_kmp_sect!(Object, KmpSection::Objects);
impl_to_kmp_sect!(RoutePoint, KmpSection::Routes);
impl_to_kmp_sect!(AreaPoint, KmpSection::Areas);
impl_to_kmp_sect!(KmpCamera, KmpSection::Cameras);
impl_to_kmp_sect!(CannonPoint, KmpSection::CannonPoints);
impl_to_kmp_sect!(BattleFinishPoint, KmpSection::BattleFinishPoints);
impl_to_kmp_sect!(TrackInfo, KmpSection::TrackInfo);
