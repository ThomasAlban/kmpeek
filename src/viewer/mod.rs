use bevy::app::Plugin;

use self::{
    camera::CameraPlugin, grid::GridPlugin, kcl_model::KclPlugin, kmp::KmpPlugin,
    mouse_picking::MousePickingPlugin, normalize::NormalizePlugin,
};

pub mod camera;
mod grid;
pub mod kcl_model;
pub mod kmp;
mod mouse_picking;
mod normalize;
pub struct ViewerPlugin;
impl Plugin for ViewerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins((
            CameraPlugin,
            KmpPlugin,
            KclPlugin,
            NormalizePlugin,
            GridPlugin,
            MousePickingPlugin,
        ));
    }
}
