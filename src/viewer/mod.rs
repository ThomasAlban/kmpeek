use bevy::app::Plugin;

use self::{
    camera::CameraPlugin, grid::GridPlugin, kcl_model::KclPlugin, kmp_model::KmpPlugin,
    normalize::NormalizePlugin,
};

pub mod camera;
mod grid;
pub mod kcl_model;
pub mod kmp_model;
mod mouse_picking;
mod normalize;
mod undo;
pub struct ViewerPlugin;
impl Plugin for ViewerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins((
            CameraPlugin,
            GridPlugin,
            KmpPlugin,
            KclPlugin,
            NormalizePlugin,
        ));
    }
}
