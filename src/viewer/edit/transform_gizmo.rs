use super::{select::Selected, EditMode};
use crate::{
    ui::viewport::ViewportInfo,
    viewer::kmp::checkpoints::{CheckpointLeft, CheckpointRight},
};
use bevy::prelude::*;
use transform_gizmo_bevy::{enum_set, GizmoMode, GizmoOptions, GizmoTarget, GizmoVisuals};

#[derive(Component)]
pub struct GizmoTransformable;

pub fn transform_gizmo_plugin(app: &mut App) {
    app.add_plugins(transform_gizmo_bevy::prelude::TransformGizmoPlugin)
        .insert_resource(GizmoOptions {
            gizmo_modes: GizmoMode::all_translate(),
            visuals: GizmoVisuals {
                gizmo_size: 125.,
                stroke_width: 8.,
                ..default()
            },
            ..default()
        })
        .add_systems(Update, update_gizmo);
}

fn update_gizmo(
    mut commands: Commands,
    edit_mode: Res<EditMode>,
    q_selected_cp: Query<(), (With<Selected>, Or<(With<CheckpointLeft>, With<CheckpointRight>)>)>,
    q_selectable: Query<(Entity, Has<Selected>, Has<GizmoTarget>), With<GizmoTransformable>>,
    mut gizmo_options: ResMut<GizmoOptions>,
    viewport_info: Res<ViewportInfo>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    // update gizmo viewport
    gizmo_options.viewport_rect = Some(viewport_info.viewport_rect);

    // update gizmo mode
    if edit_mode.is_changed() {
        match *edit_mode {
            EditMode::Translate => gizmo_options.gizmo_modes = GizmoMode::all_translate(),
            EditMode::Rotate => gizmo_options.gizmo_modes = GizmoMode::all_rotate(),
            _ => (),
        };
        // if we have checkpoints selected
        if !q_selected_cp.is_empty() {
            match *edit_mode {
                EditMode::Translate => {
                    gizmo_options.gizmo_modes =
                        enum_set!(GizmoMode::TranslateX | GizmoMode::TranslateZ | GizmoMode::TranslateXZ)
                }
                EditMode::Rotate => gizmo_options.gizmo_modes = enum_set!(GizmoMode::RotateY),
                _ => (),
            };
        }
    }
    // update gizmo targets
    let mut remove_all_targets = false;
    if *edit_mode != EditMode::Translate && *edit_mode != EditMode::Rotate {
        if edit_mode.is_changed() {
            remove_all_targets = true;
        } else {
            return;
        }
    }
    for (e, is_selected, is_gizmo_target) in q_selectable.iter() {
        if remove_all_targets {
            commands.entity(e).remove::<GizmoTarget>();
            continue;
        }
        if is_selected && !is_gizmo_target {
            commands.entity(e).insert(GizmoTarget::default());
        } else if !is_selected && is_gizmo_target {
            commands.entity(e).remove::<GizmoTarget>();
        }
    }
    // update whether snapping is enabled
    gizmo_options.snapping = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
}
