mod components;
mod resources;
mod systems;

use bevy::prelude::*;
use bevy_mod_picking::DefaultPickingPlugins;
pub use components::*;
pub use resources::{CameraMode, CameraSettings};
use systems::*;

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>()
            .add_plugins(DefaultPickingPlugins)
            .add_startup_system(camera_setup)
            .add_system(cursor_grab)
            .add_system(fly_cam_look)
            .add_system(fly_cam_move)
            .add_system(orbit_cam);
    }
}
