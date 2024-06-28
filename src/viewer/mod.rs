use bevy::app::App;

use self::{
    camera::camera_plugin, edit::mouse_picking_plugin, grid::grid_plugin, kcl_model::kcl_plugin, kmp::kmp_plugin,
    normalize::normalize_plugin,
};

pub mod camera;
pub mod edit;
mod grid;
pub mod kcl_model;
pub mod kmp;
mod normalize;

pub fn viewer_plugin(app: &mut App) {
    app.add_plugins((
        camera_plugin,
        kmp_plugin,
        kcl_plugin,
        normalize_plugin,
        grid_plugin,
        mouse_picking_plugin,
    ));
}
