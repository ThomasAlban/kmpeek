use bevy::ecs::system::Resource;
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Display, EnumString, IntoStaticStr, EnumIter, Default, PartialEq, Clone, Copy)]
pub enum KmpModelSections {
    #[default]
    #[strum(serialize = "Start Points", props(num = "0"))]
    StartPoints,
    #[strum(serialize = "Enemy Paths")]
    EnemyPaths,
    #[strum(serialize = "Item Paths")]
    ItemPaths,
    Checkpoints,
    #[strum(serialize = "Respawn Points")]
    RespawnPoints,
    Objects,
    Areas,
    Cameras,
    #[strum(serialize = "Cannon Points")]
    CannonPoints,
    #[strum(serialize = "Battle Finish Points")]
    BattleFinishPoints,
}

#[derive(Resource, Default)]
pub struct KmpEditMode(pub KmpModelSections);

impl From<KmpModelSections> for usize {
    fn from(value: KmpModelSections) -> Self {
        use KmpModelSections::*;
        match value {
            StartPoints => 0,
            EnemyPaths => 1,
            ItemPaths => 2,
            Checkpoints => 3,
            RespawnPoints => 4,
            Objects => 5,
            Areas => 6,
            Cameras => 7,
            CannonPoints => 8,
            BattleFinishPoints => 9,
        }
    }
}
