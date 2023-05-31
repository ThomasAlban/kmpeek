// this camera control is a modified version of the one from bevy_flycam
// https://github.com/sburris0/bevy_flycam

mod components;
mod resources;
mod systems;

use bevy::prelude::*;
use bevy_mod_picking::DefaultPickingPlugins;

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<resources::InputState>()
            .init_resource::<resources::MovementSettings>()
            .init_resource::<resources::KeyBindings>()
            .add_plugins(DefaultPickingPlugins)
            .add_startup_system(systems::camera_setup)
            .add_system(systems::camera_move)
            .add_system(systems::camera_look)
            .add_system(systems::cursor_grab);
    }
}
