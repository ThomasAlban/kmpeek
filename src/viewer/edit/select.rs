use super::area_gizmo::AreaGizmoOptions;
use super::create_delete::JustCreatedPoint;
use super::EditMode;
use crate::ui::keybinds::{Modifier, ModifiersPressed};
use crate::ui::ui_state::KmpVisibility;
use crate::ui::update_ui::UpdateUiSet;
use crate::ui::viewport::ViewportInfo;
use crate::util::{ui_viewport_to_ndc, world_to_ui_viewport, RaycastFromCam};
use crate::viewer::camera::Gizmo2dCam;
use crate::viewer::kmp::components::KmpSelectablePoint;
use crate::viewer::kmp::sections::KmpEditModeChange;
use bevy::prelude::*;
use bevy_mod_outline::*;
use bevy_mod_raycast::prelude::*;
use transform_gizmo_bevy::GizmoTarget;

#[derive(SystemSet, Debug, PartialEq, Eq, Hash, Clone)]
pub struct SelectSet;

pub fn select_plugin(app: &mut App) {
    app.init_resource::<SelectBox>()
        .add_systems(Update, (select, select_box, select_all).in_set(SelectSet))
        .add_systems(Update, update_outlines.after(SelectSet))
        .add_systems(
            Update,
            (
                deselect_if_not_visible.run_if(resource_changed::<KmpVisibility>),
                deselect_on_mode_change.after(UpdateUiSet),
            ),
        );
}

#[derive(Component, Default)]
pub struct Selected;

fn select(
    viewport_info: Res<ViewportInfo>,
    q_window: Query<&Window>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&mut Camera, &GlobalTransform), Without<Gizmo2dCam>>,
    q_gizmos: Query<&GizmoTarget>,
    mut raycast: Raycast,
    q_kmp_section: Query<&KmpSelectablePoint>,
    mut commands: Commands,
    area_gizmo_opts: Res<AreaGizmoOptions>,
    q_selected: Query<Entity, With<Selected>>,
    mut ev_just_created_point: EventReader<JustCreatedPoint>,
) {
    if !viewport_info.mouse_in_viewport
        || viewport_info.mouse_on_overlayed_ui
        || !mouse_buttons.just_pressed(MouseButton::Left)
        || (ev_just_created_point.is_empty() && (keys.pressed(KeyCode::AltLeft)) || keys.pressed(KeyCode::AltRight))
        || area_gizmo_opts.mouse_hovering
        || q_gizmos.iter().any(|x| x.is_focused())
    {
        return;
    }

    let Some(mouse_pos) = q_window.single().cursor_position() else {
        return;
    };

    let shift_key_down = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // get the active camera
    let cam = q_camera.iter().find(|cam| cam.0.is_active).unwrap();

    let mouse_pos_ndc = ui_viewport_to_ndc(mouse_pos, viewport_info.viewport_rect);

    let intersections = RaycastFromCam::new(cam, mouse_pos_ndc, &mut raycast)
        .filter(&|e| q_kmp_section.contains(e))
        .cast();
    let intersection = intersections.first();

    // deselect everything if we already have something selected but don't have the shift key down
    if intersection.is_some() && !shift_key_down {
        for selected in q_selected.iter() {
            commands.entity(selected).remove::<Selected>();
        }
    }
    // select the entity
    if let Some((to_select, _)) = intersection {
        commands.entity(*to_select).insert(Selected);
    } else if !shift_key_down {
        // if we just randomly clicked on nothing then deselect everything
        for selected in q_selected.iter() {
            commands.entity(selected).remove::<Selected>();
        }
    }
    for created_point in ev_just_created_point.read() {
        commands.entity(created_point.0).insert(Selected);
    }
}

fn select_all(
    mut commands: Commands,
    q_selectable: Query<(Entity, &Visibility), With<KmpSelectablePoint>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !keys.keybind_pressed([Modifier::Ctrl], [KeyCode::KeyA]) {
        return;
    }

    for (e, visibility) in q_selectable.iter() {
        if *visibility == Visibility::Visible {
            commands.entity(e).insert(Selected);
        }
    }
}

fn deselect_if_not_visible(mut commands: Commands, q_selected: Query<(Entity, &Visibility), With<Selected>>) {
    for (e, selected) in q_selected.iter() {
        if selected != Visibility::Visible {
            commands.entity(e).remove::<Selected>();
        }
    }
}

fn deselect_on_mode_change(
    ev_mode_change: EventReader<KmpEditModeChange>,
    mut commands: Commands,
    q_selected: Query<Entity, With<Selected>>,
) {
    if ev_mode_change.is_empty() {
        return;
    }
    for e in q_selected.iter() {
        commands.entity(e).remove::<Selected>();
    }
}

#[derive(Resource, Default)]
pub struct SelectBox(pub Option<Rect>);
impl SelectBox {
    /// How much we have to move the mouse before we actually start making a select box
    const LENIENCY_BEFORE_SELECT: f32 = 3.;
}

// this handles working out the select box rectangle and actually selecting stuff (the visuals for the box are handled in the UI section)
fn select_box(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_window: Query<&Window>,
    edit_mode: Res<EditMode>,
    viewport_info: Res<ViewportInfo>,
    q_selectable: Query<(&Transform, Entity, &Visibility, Has<Selected>), With<KmpSelectablePoint>>,
    q_camera: Query<(&Camera, &GlobalTransform), Without<Gizmo2dCam>>,
    mut commands: Commands,
    mut select_box: ResMut<SelectBox>,
    mut initial_mouse_pos: Local<Vec2>,
) {
    if *edit_mode != EditMode::SelectBox {
        return;
    }

    let window = q_window.single();
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };

    // save the initial scaled/unscaled mouse pos into local variables (so that they can be used for one corner of the select box)
    if mouse_buttons.just_pressed(MouseButton::Left) {
        *initial_mouse_pos = mouse_pos;
    }

    if mouse_buttons.pressed(MouseButton::Left)
        && initial_mouse_pos.distance(mouse_pos) > SelectBox::LENIENCY_BEFORE_SELECT
    {
        // delete the select box if mouse isn't in viewport
        if !viewport_info.mouse_in_viewport {
            *select_box = SelectBox::default();
            return;
        }

        // set the select box with the initial mouse pos and the current mouse pos as the 2 corners
        *select_box = SelectBox(Some(Rect::from_corners(*initial_mouse_pos, mouse_pos)));
    }

    // when we release the mouse button, we actually select stuff
    if mouse_buttons.just_released(MouseButton::Left) {
        let Some(select_rect) = select_box.0 else {
            return;
        };
        // get the active camera
        let cam = q_camera.iter().find(|cam| cam.0.is_active).unwrap();

        // select stuff
        for selectable in q_selectable.iter() {
            if selectable.2 != Visibility::Visible || selectable.3 {
                continue;
            }
            let Some(viewport_pos) = world_to_ui_viewport(cam, viewport_info.viewport_rect, selectable.0.translation)
            else {
                continue;
            };
            if select_rect.contains(viewport_pos) {
                commands.entity(selectable.1).insert(Selected);
            }
        }
        // reset the select box after we've selected stuff
        *select_box = SelectBox::default();
    }
}

// put outlines on any entities which are selected, and remove them if they aren't selected
fn update_outlines(
    q_entities: Query<(Entity, Has<Selected>, &Visibility), With<KmpSelectablePoint>>,
    mut q_outline: Query<&mut OutlineVolume>,
) {
    for (entity, is_selected, visibility) in q_entities.iter() {
        let Ok(mut outline) = q_outline.get_mut(entity) else {
            continue;
        };
        outline.visible = is_selected;
        if visibility != Visibility::Visible {
            outline.visible = false;
        }
    }
}
