mod components;
mod resources;
mod systems;

use bevy::prelude::*;
use bevy_infinite_grid::InfiniteGridPlugin;
pub use components::*;
pub use resources::{CameraMode, CameraSettings, FlySettings, OrbitSettings, TopDownSettings};
use systems::*;

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>()
            .add_startup_system(camera_setup)
            .add_plugin(InfiniteGridPlugin)
            .add_system(cursor_grab)
            .add_system(update_active_camera)
            .add_system(fly_cam_look)
            .add_system(fly_cam_move)
            .add_system(orbit_cam)
            .add_system(top_down_cam);
    }
}
