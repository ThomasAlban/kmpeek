pub mod area_gizmo;
pub mod create_delete;
pub mod link_select_mode;
pub mod link_unlink_path;
pub mod select;
pub mod transform_gizmo;
pub mod tweak;

use self::{
    area_gizmo::area_gizmo_plugin, create_delete::create_delete_plugin, link_unlink_path::link_unlink_plugin,
    select::select_plugin, transform_gizmo::transform_gizmo_plugin, tweak::tweak_plugin,
};
use bevy::prelude::*;
use bevy_mod_outline::OutlinePlugin;
use link_select_mode::link_select_mode_plugin;
use strum_macros::EnumIter;

pub fn edit_plugin(app: &mut App) {
    app.add_plugins((
        OutlinePlugin,
        transform_gizmo_plugin,
        area_gizmo_plugin,
        select_plugin,
        create_delete_plugin,
        link_unlink_plugin,
        tweak_plugin,
        link_select_mode_plugin,
    ))
    .init_resource::<EditMode>();
}

#[derive(Resource, Default, PartialEq, EnumIter, Debug)]
pub enum EditMode {
    #[default]
    Tweak,
    SelectBox,
    Translate,
    Rotate,
}
