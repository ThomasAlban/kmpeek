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
