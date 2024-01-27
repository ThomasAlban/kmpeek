pub mod create_delete;
pub mod gizmo;
pub mod kcl_snap;
pub mod select;

use self::{
    create_delete::{create_point, delete_point},
    gizmo::TransformGizmoPlugin,
    kcl_snap::snap_to_kcl,
    select::{deselect_if_not_visible, select, select_box, update_outlines, SelectBox, SelectSet},
};
use crate::ui::update_ui::UpdateUiSet;
use bevy::prelude::*;
use bevy_mod_outline::OutlinePlugin;
use bevy_mod_raycast::DefaultRaycastingPlugin;
use strum_macros::EnumIter;

use super::kmp::KmpVisibilityUpdate;

pub struct MousePickingPlugin;
impl Plugin for MousePickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DefaultRaycastingPlugin, OutlinePlugin, TransformGizmoPlugin))
            .init_resource::<EditMode>()
            .init_resource::<SelectBox>()
            .add_systems(
                Update,
                (select, select_box, update_outlines)
                    .chain()
                    .in_set(SelectSet)
                    .after(UpdateUiSet),
            )
            .add_systems(
                Update,
                (snap_to_kcl, create_point, delete_point).after(SelectSet),
            )
            .add_systems(
                Update,
                deselect_if_not_visible.run_if(on_event::<KmpVisibilityUpdate>()),
            );
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
