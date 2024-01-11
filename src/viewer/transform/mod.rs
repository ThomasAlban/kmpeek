pub mod gizmo;
pub mod kcl_snap;
pub mod select;

use self::{
    gizmo::TransformGizmoPlugin,
    kcl_snap::snap_to_kcl,
    select::{deselect_on_mode_change, select, SelectSet},
};
use crate::ui::{ui_state::AppModeChanged, update_ui::UpdateUiSet};
use bevy::prelude::*;
use bevy_mod_outline::OutlinePlugin;
use bevy_mod_raycast::DefaultRaycastingPlugin;
use strum_macros::EnumIter;

use super::normalize::UpdateNormalizeSet;

pub struct MousePickingPlugin;
impl Plugin for MousePickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DefaultRaycastingPlugin, OutlinePlugin, TransformGizmoPlugin))
            .init_resource::<TransformMode>()
            .add_systems(
                Update,
                (apply_deferred, select, apply_deferred)
                    .chain()
                    .in_set(SelectSet)
                    .after(UpdateUiSet)
                    .after(UpdateNormalizeSet),
            )
            .add_systems(Update, snap_to_kcl.after(SelectSet))
            .add_systems(
                Update,
                deselect_on_mode_change.run_if(on_event::<AppModeChanged>()),
            );
    }
}

#[derive(Resource, Default, PartialEq, EnumIter)]
pub enum TransformMode {
    #[default]
    KclSnap,
    GizmoTranslate,
    GizmoRotate,
}
