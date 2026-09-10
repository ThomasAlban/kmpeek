use std::any::TypeId;

use bevy::{ecs::resource::Resource, prelude::*};
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

use super::{
    AreaPoint, BattleFinishPoint, CannonPoint, Checkpoint, EnemyPathPoint, ItemPathPoint, KmpCamera, Object,
    RespawnPoint, RoutePoint, StartPoint, TrackInfo,
};

pub fn section_plugin(app: &mut App) {
    app.init_resource::<KmpEditMode>();
}

#[derive(Resource, Display, EnumString, IntoStaticStr, EnumIter, Default, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum KmpEditMode {
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
impl KmpEditMode {
    pub fn to_type_id(self) -> TypeId {
        match self {
            Self::StartPoints => TypeId::of::<StartPoint>(),
            Self::EnemyPaths => TypeId::of::<EnemyPathPoint>(),
            Self::ItemPaths => TypeId::of::<ItemPathPoint>(),
            Self::Checkpoints => TypeId::of::<Checkpoint>(),
            Self::RespawnPoints => TypeId::of::<RespawnPoint>(),
            Self::Objects => TypeId::of::<Object>(),
            Self::Routes => TypeId::of::<RoutePoint>(),
            Self::Areas => TypeId::of::<AreaPoint>(),
            Self::Cameras => TypeId::of::<KmpCamera>(),
            Self::CannonPoints => TypeId::of::<CannonPoint>(),
            Self::BattleFinishPoints => TypeId::of::<BattleFinishPoint>(),
            Self::TrackInfo => TypeId::of::<TrackInfo>(),
        }
    }
    /// Panics if the inputted type does not map to an enum variant
    pub fn from_type<T: 'static>() -> Self {
        let t = TypeId::of::<T>();
        if t == TypeId::of::<StartPoint>() {
            Self::StartPoints
        } else if t == TypeId::of::<EnemyPathPoint>() {
            Self::EnemyPaths
        } else if t == TypeId::of::<ItemPathPoint>() {
            Self::ItemPaths
        } else if t == TypeId::of::<Checkpoint>() {
            Self::Checkpoints
        } else if t == TypeId::of::<RespawnPoint>() {
            Self::RespawnPoints
        } else if t == TypeId::of::<Object>() {
            Self::Objects
        } else if t == TypeId::of::<RoutePoint>() {
            Self::Routes
        } else if t == TypeId::of::<AreaPoint>() {
            Self::Areas
        } else if t == TypeId::of::<KmpCamera>() {
            Self::Cameras
        } else if t == TypeId::of::<CannonPoint>() {
            Self::CannonPoints
        } else if t == TypeId::of::<BattleFinishPoint>() {
            Self::BattleFinishPoints
        } else if t == TypeId::of::<TrackInfo>() {
            Self::TrackInfo
        } else {
            panic!("Incorrect type input supplied to KmpSection::from_type");
        }
    }
    pub fn in_mode<T: 'static>(&self) -> bool {
        self.to_type_id() == TypeId::of::<T>()
    }
    pub fn set_mode<T: 'static>(&mut self) {
        *self = Self::from_type::<T>();
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
        $app.add_message::<$ev<StartPoint>>()
            .add_message::<$ev<EnemyPathPoint>>()
            .add_message::<$ev<ItemPathPoint>>()
            .add_message::<$ev<Checkpoint>>()
            .add_message::<$ev<RespawnPoint>>()
            .add_message::<$ev<Object>>()
            .add_message::<$ev<RoutePoint>>()
            .add_message::<$ev<AreaPoint>>()
            .add_message::<$ev<KmpCamera>>()
            .add_message::<$ev<CannonPoint>>()
            .add_message::<$ev<BattleFinishPoint>>()
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
