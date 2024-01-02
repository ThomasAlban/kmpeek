use bevy::app::Plugin;

use self::{
    camera::CameraPlugin, grid::GridPlugin, kcl_model::KclPlugin, kmp::KmpPlugin,
    normalize::NormalizePlugin,
};

pub mod camera;
mod grid;
pub mod kcl_model;
pub mod kmp;
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
        ));
    }
}