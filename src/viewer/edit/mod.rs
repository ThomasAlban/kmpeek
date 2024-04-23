pub mod area_gizmo;
pub mod create_delete;
pub mod kcl_snap;
pub mod link_unlink;
pub mod select;
pub mod transform_gizmo;

use self::{
    area_gizmo::AreaGizmoPlugin, create_delete::CreateDeletePlugin, kcl_snap::KclSnapPlugin,
    link_unlink::LinkUnlinkPlugin, select::SelectPlugin, transform_gizmo::TransformGizmoPlugin,
};
use bevy::prelude::*;
use bevy_mod_outline::OutlinePlugin;
use bevy_mod_raycast::DefaultRaycastingPlugin;
use strum_macros::EnumIter;

pub struct MousePickingPlugin;
impl Plugin for MousePickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultRaycastingPlugin,
            OutlinePlugin,
            TransformGizmoPlugin,
            AreaGizmoPlugin,
            SelectPlugin,
            CreateDeletePlugin,
            LinkUnlinkPlugin,
            KclSnapPlugin,
        ))
        .init_resource::<EditMode>();
    }
}

#[derive(Resource, Default, PartialEq, EnumIter, Debug)]
pub enum EditMode {
    #[default]
    Tweak,
    SelectBox,
    Translate,
    Rotate,
}
