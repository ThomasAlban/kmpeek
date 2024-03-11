use super::gizmo::GizmoOptions;
use super::EditMode;
use crate::ui::ui_state::{MouseInViewport, ViewportRect};
use crate::util::{cast_ray_from_cam, ui_viewport_to_ndc, world_to_ui_viewport};
use crate::viewer::kmp::area::BoxGizmoOptions;
use crate::viewer::kmp::components::KmpSelectablePoint;
use crate::viewer::kmp::sections::KmpEditMode;
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use bevy_mod_outline::*;
use bevy_mod_raycast::prelude::*;

#[derive(Component, Default)]
pub struct Selected;

pub fn select(
    mouse_in_viewport: Res<MouseInViewport>,
    viewport_rect: Res<ViewportRect>,
    q_window: Query<&Window>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut raycast: Raycast,
    q_kmp_section: Query<&KmpSelectablePoint>,
    mut commands: Commands,
    gizmo_options: Res<GizmoOptions>,
    box_gizmo_options: Res<BoxGizmoOptions>,
    q_selected: Query<Entity, With<Selected>>,
    q_visibility: Query<&Visibility>,
    mut contexts: EguiContexts,
) {
    if !mouse_in_viewport.0
        || !mouse_buttons.just_pressed(MouseButton::Left)
        || contexts.ctx_mut().wants_pointer_input()
    {
        return;
    }
    let window = q_window.single();
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };

    if gizmo_options.last_result.is_some() || box_gizmo_options.mouse_interacting {
        return;
    }
    let shift_key_down = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // get the active camera
    let cam = q_camera.iter().find(|cam| cam.0.is_active).unwrap();

    let mouse_pos_ndc = ui_viewport_to_ndc(mouse_pos, viewport_rect.0);

    let intersections = cast_ray_from_cam(cam, mouse_pos_ndc, &mut raycast, |e| {
        let visibility = q_visibility.get(e).unwrap();
        q_kmp_section.contains(e) && visibility == Visibility::Visible
    });
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
}

pub fn deselect_if_not_visible(mut commands: Commands, q_selected: Query<(Entity, &Visibility), With<Selected>>) {
    for (e, selected) in q_selected.iter() {
        if selected != Visibility::Visible {
            commands.entity(e).remove::<Selected>();
        }
    }
}

pub fn deselect_on_mode_change(
    edit_mode: Res<KmpEditMode>,
    mut commands: Commands,
    q_selected: Query<Entity, With<Selected>>,
) {
    if !edit_mode.is_changed() {
        return;
    }
    for e in q_selected.iter() {
        commands.entity(e).remove::<Selected>();
    }
}

#[derive(Resource, Default)]
pub struct SelectBox(pub Option<Rect>);
impl SelectBox {
    // how much we have to move the mouse before we actually start making a select box
    const LENIENCY_BEFORE_SELECT: f32 = 3.;
}

// this handles working out the select box rectangle and actually selecting stuff (the visuals for the box are handled in the UI section)
pub fn select_box(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_window: Query<&Window>,
    edit_mode: Res<EditMode>,
    mouse_in_viewport: Res<MouseInViewport>,
    q_selectable: Query<(&Transform, Entity, &Visibility, Has<Selected>), With<KmpSelectablePoint>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut commands: Commands,
    mut select_box: ResMut<SelectBox>,
    viewport_rect: Res<ViewportRect>,
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
        if !mouse_in_viewport.0 {
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
            let Some(viewport_pos) = world_to_ui_viewport(cam, viewport_rect.0, selectable.0.translation) else {
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
pub fn update_outlines(
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
