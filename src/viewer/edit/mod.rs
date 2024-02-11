pub mod create_delete;
pub mod gizmo;
pub mod kcl_snap;
pub mod select;

use crate::ui::update_ui::UpdateUiSet;

use self::{
    create_delete::{create_point, delete_point},
    gizmo::TransformGizmoPlugin,
    kcl_snap::snap_to_kcl,
    select::{deselect_if_not_visible, select, select_box, update_outlines, SelectBox},
};
use super::kmp::KmpVisibilityUpdate;
use bevy::prelude::*;
use bevy_mod_outline::OutlinePlugin;
use bevy_mod_raycast::DefaultRaycastingPlugin;
use strum_macros::EnumIter;

pub struct MousePickingPlugin;
impl Plugin for MousePickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DefaultRaycastingPlugin, OutlinePlugin, TransformGizmoPlugin))
            .init_resource::<EditMode>()
            .init_resource::<SelectBox>()
            .add_systems(
                Update,
                (
                    // select stuff and outline it
                    (select, select_box),
                    update_outlines,
                    apply_deferred,
                    // create/delete/drag points around now that we know what is selected
                    (create_point, delete_point, snap_to_kcl),
                    apply_deferred,
                )
                    .chain()
                    // after UI so that if we interact with the gizmo we can not deselect stuff
                    .after(UpdateUiSet),
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
