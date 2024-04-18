pub mod create_delete;
pub mod gizmo;
pub mod kcl_snap;
pub mod link_unlink;
pub mod select;

use crate::ui::{ui_state::KmpVisibility, update_ui::UpdateUiSet};

use self::{
    create_delete::{create_point, delete_point, JustCreatedPoint},
    gizmo::TransformGizmoPlugin,
    kcl_snap::snap_to_kcl,
    link_unlink::{link_points, unlink_points},
    select::{deselect_if_not_visible, deselect_on_mode_change, select, select_box, update_outlines, SelectBox},
};
use bevy::prelude::*;
use bevy_mod_outline::OutlinePlugin;
use bevy_mod_raycast::DefaultRaycastingPlugin;
use strum_macros::EnumIter;

pub struct MousePickingPlugin;
impl Plugin for MousePickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DefaultRaycastingPlugin, OutlinePlugin, TransformGizmoPlugin))
            .add_event::<JustCreatedPoint>()
            .init_resource::<EditMode>()
            .init_resource::<SelectBox>()
            .add_systems(
                Update,
                (
                    create_point,
                    (link_points, unlink_points),
                    // select stuff and outline it
                    (select, select_box),
                    update_outlines,
                    // create/delete/drag points around now that we know what is selected
                    (delete_point, snap_to_kcl),
                )
                    .chain()
                    // after UI so that if we interact with the gizmo we can not deselect stuff
                    .after(UpdateUiSet),
            )
            .add_systems(
                Update,
                (
                    deselect_if_not_visible.run_if(resource_changed::<KmpVisibility>),
                    deselect_on_mode_change.after(UpdateUiSet),
                ),
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
