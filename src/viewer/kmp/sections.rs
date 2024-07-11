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

#[derive(SystemParam)]
pub struct KmpEditModeOptions<'w, 's> {
    start_point: Option<Res<'w, KmpEditMode<StartPoint>>>,
    enemy_path: Option<Res<'w, KmpEditMode<EnemyPathPoint>>>,
    item_path: Option<Res<'w, KmpEditMode<ItemPathPoint>>>,
    checkpoint: Option<Res<'w, KmpEditMode<Checkpoint>>>,
    respawn_point: Option<Res<'w, KmpEditMode<RespawnPoint>>>,
    object: Option<Res<'w, KmpEditMode<Object>>>,
    route: Option<Res<'w, KmpEditMode<RoutePoint>>>,
    area: Option<Res<'w, KmpEditMode<AreaPoint>>>,
    camera: Option<Res<'w, KmpEditMode<KmpCamera>>>,
    cannon_point: Option<Res<'w, KmpEditMode<CannonPoint>>>,
    battle_finish_point: Option<Res<'w, KmpEditMode<BattleFinishPoint>>>,
    commands: Commands<'w, 's>,
    ev_mode_change: EventWriter<'w, KmpEditModeChange>,
    // etc
}

impl<'w, 's> KmpEditModeOptions<'w, 's> {
    pub fn get_kmp_section(&self) -> KmpSection {
        if self.start_point.is_some() {
            KmpSection::StartPoints
        } else if self.enemy_path.is_some() {
            KmpSection::EnemyPaths
        } else if self.item_path.is_some() {
            KmpSection::ItemPaths
        } else if self.checkpoint.is_some() {
            KmpSection::Checkpoints
        } else if self.respawn_point.is_some() {
            KmpSection::RespawnPoints
        } else if self.object.is_some() {
            KmpSection::Objects
        } else if self.route.is_some() {
            KmpSection::Routes
        } else if self.area.is_some() {
            KmpSection::Areas
        } else if self.camera.is_some() {
            KmpSection::Cameras
        } else if self.cannon_point.is_some() {
            KmpSection::CannonPoints
        } else if self.battle_finish_point.is_some() {
            KmpSection::BattleFinishPoints
        } else {
            KmpSection::TrackInfo
        }
    }
    fn remove_all_modes(&mut self) {
        self.commands.remove_resource::<KmpEditMode<StartPoint>>();
        self.commands.remove_resource::<KmpEditMode<EnemyPathPoint>>();
        self.commands.remove_resource::<KmpEditMode<ItemPathPoint>>();
        self.commands.remove_resource::<KmpEditMode<Checkpoint>>();
        self.commands.remove_resource::<KmpEditMode<RespawnPoint>>();
        self.commands.remove_resource::<KmpEditMode<Object>>();
        self.commands.remove_resource::<KmpEditMode<RoutePoint>>();
        self.commands.remove_resource::<KmpEditMode<AreaPoint>>();
        self.commands.remove_resource::<KmpEditMode<KmpCamera>>();
        self.commands.remove_resource::<KmpEditMode<CannonPoint>>();
        self.commands.remove_resource::<KmpEditMode<BattleFinishPoint>>();
        self.commands.remove_resource::<KmpEditMode<TrackInfo>>();
    }
    pub fn change_mode<T: Component + ToKmpSection>(&mut self) {
        self.remove_all_modes();
        self.commands.insert_resource(KmpEditMode::<T>::default());
        self.ev_mode_change.send_default();
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
