use bevy::prelude::*;

mod components;
mod resources;
mod systems;

pub use resources::{Kcl, KclFlag, KCL_COLOURS};
use systems::*;
pub struct KclPlugin;

impl Plugin for KclPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_model).add_system(update_kcl_model);
    }
}
