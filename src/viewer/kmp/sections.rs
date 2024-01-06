use bevy::ecs::component::Component;
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Display, EnumString, IntoStaticStr, EnumIter, Component)]
pub enum KmpSections {
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
    Area,
    Cameras,
    #[strum(serialize = "Cannon Points")]
    CannonPoints,
    #[strum(serialize = "Battle Finish Points")]
    BattleFinishPoints,
}

impl From<KmpSections> for usize {
    fn from(value: KmpSections) -> Self {
        use KmpSections::*;
        match value {
            StartPoints => 0,
            EnemyPaths => 1,
            ItemPaths => 2,
            Checkpoints => 3,
            RespawnPoints => 4,
            Objects => 5,
            Routes => 6,
            Area => 7,
            Cameras => 8,
            CannonPoints => 9,
            BattleFinishPoints => 10,
        }
    }
}
