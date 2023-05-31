use bevy::prelude::*;

mod components;
mod resources;
mod systems;

pub use resources::Kmp;
pub struct KmpPlugin;

impl Plugin for KmpPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(systems::spawn_model);
    }
}
