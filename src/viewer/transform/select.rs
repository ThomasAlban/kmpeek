use super::gizmo::GizmoOptions;
use super::EditMode;
use crate::ui::ui_state::{MouseInViewport, ViewportRect};
use crate::viewer::kmp::components::KmpSection;
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::EguiContexts;
use bevy_mod_outline::*;
use bevy_mod_raycast::prelude::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct SelectSet;

pub fn scale_viewport_pos(viewport_pos: Vec2, window: &Window, viewport_rect: Rect) -> Vec2 {
    // make (0,0) be the top left corner of the viewport
    let mut scaled_viewport_pos = viewport_pos - viewport_rect.min;
    scaled_viewport_pos = scaled_viewport_pos.clamp(Vec2::ZERO, viewport_rect.max);
    scaled_viewport_pos *= window.scale_factor() as f32;
    scaled_viewport_pos
}

pub fn get_ray_from_cam(cam: (&Camera, &GlobalTransform), scaled_viewport_pos: Vec2) -> Ray3d {
    cam.0
        .viewport_to_world(cam.1, scaled_viewport_pos)
        .map(Ray3d::from)
        .unwrap()
}

pub fn cast_ray_from_cam(
    cam: (&Camera, &GlobalTransform),
    scaled_viewport_pos: Vec2,
    raycast: &mut Raycast,
    filter: impl Fn(Entity) -> bool,
) -> Vec<(Entity, IntersectionData)> {
    let ray = get_ray_from_cam(cam, scaled_viewport_pos);

    let raycast_result = raycast
        .cast_ray(ray, &RaycastSettings::default().with_filter(&filter))
        .to_vec();

    raycast_result
}

#[derive(Component, Default)]
pub struct Selected;

pub fn select(
    mouse_in_viewport: Res<MouseInViewport>,
    viewport_rect: Res<ViewportRect>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    keys: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut raycast: Raycast,
    q_kmp_section: Query<&KmpSection>,
    mut commands: Commands,
    gizmo: Res<GizmoOptions>,
    mut q_outline: Query<&mut OutlineVolume>,
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
    let window = q_window.get_single().unwrap();
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };
    if gizmo.last_result.is_some() {
        return;
    }
    let shift_key_down = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // get the active camera
    let cam = q_camera
        .iter()
        .filter(|cam| cam.0.is_active)
        .collect::<Vec<(&Camera, &GlobalTransform)>>()[0];

    let scaled_mouse_pos = scale_viewport_pos(mouse_pos, window, viewport_rect.0);
    // send out a ray
    let intersections = cast_ray_from_cam(cam, scaled_mouse_pos, &mut raycast, |e| {
        let visibility = q_visibility.get(e).unwrap();
        q_kmp_section.contains(e) && visibility == Visibility::Visible
    });
    let intersection = intersections.first();

    // deselect everything if we already have something selected but don't have the shift key down
    if intersection.is_some() && !shift_key_down {
        for selected in q_selected.iter() {
            commands.entity(selected).remove::<Selected>();
            // remove the outline
            if let Ok(mut outline) = q_outline.get_mut(selected) {
                outline.visible = false;
            }
        }
    }
    // select the entity
    if let Some((to_select, _)) = intersection {
        // get the parent entity
        // set the entity as a child of the transform parent
        commands.entity(*to_select).insert(Selected);
        // add the outline
        if let Ok(mut outline) = q_outline.get_mut(*to_select) {
            outline.visible = true;
        }
    } else if !shift_key_down {
        // if we just randomly clicked on nothing then deselect everything
        for selected in q_selected.iter() {
            commands.entity(selected).remove::<Selected>();
            // remove the outline
            if let Ok(mut outline) = q_outline.get_mut(selected) {
                outline.visible = false;
            }
        }
    }
}

pub fn deselect_if_not_visible(
    mut commands: Commands,
    q_selected: Query<(Entity, &Visibility), With<Selected>>,
    mut q_outline: Query<&mut OutlineVolume>,
) {
    // deselect any entity that isn't visible
    for (e, selected) in q_selected.iter() {
        if selected != Visibility::Visible {
            commands.entity(e).remove::<Selected>();
            // remove the outline
            if let Ok(mut outline) = q_outline.get_mut(e) {
                outline.visible = false;
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct SelectBox {
    pub scaled: Option<Rect>,
    pub unscaled: Option<Rect>,
}

pub fn select_box(
    mouse_buttons: Res<Input<MouseButton>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    viewport_rect: Res<ViewportRect>,
    edit_mode: Res<EditMode>,
    mouse_in_viewport: Res<MouseInViewport>,
    q_selectable: Query<(&Transform, Entity, &Visibility), With<KmpSection>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut q_outline: Query<&mut OutlineVolume>,
    mut commands: Commands,
    mut select_box: ResMut<SelectBox>,

    mut initial_scaled_mouse_pos: Local<Vec2>,
    mut initial_unscaled_mouse_pos: Local<Vec2>,
) {
    if *edit_mode != EditMode::SelectBox {
        return;
    }

    let window = q_window.single();
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };

    let scaled_mouse_pos = scale_viewport_pos(mouse_pos, window, viewport_rect.0);

    if mouse_buttons.just_pressed(MouseButton::Left) {
        *initial_scaled_mouse_pos = scaled_mouse_pos;
        *initial_unscaled_mouse_pos = mouse_pos;
    }

    if mouse_buttons.pressed(MouseButton::Left)
        && initial_scaled_mouse_pos.distance(scaled_mouse_pos) > 3.
    {
        if !mouse_in_viewport.0 {
            *select_box = SelectBox {
                scaled: None,
                unscaled: None,
            };
            return;
        }
        *select_box = SelectBox {
            scaled: Some(Rect::from_corners(
                *initial_scaled_mouse_pos,
                scaled_mouse_pos,
            )),
            unscaled: Some(Rect::from_corners(*initial_unscaled_mouse_pos, mouse_pos)),
        };
    }

    if mouse_buttons.just_released(MouseButton::Left) {
        let Some(select_rect) = select_box.scaled else {
            return;
        };
        // get the active camera
        let cam = q_camera
            .iter()
            .filter(|cam| cam.0.is_active)
            .collect::<Vec<(&Camera, &GlobalTransform)>>()[0];

        // select stuff
        for selectable in q_selectable.iter() {
            if selectable.2 != Visibility::Visible {
                continue;
            }
            let Some(viewport_pos) = cam.0.world_to_viewport(cam.1, selectable.0.translation)
            else {
                continue;
            };

            if select_rect.contains(viewport_pos) {
                commands.entity(selectable.1).insert(Selected);
                // add the outline
                if let Ok(mut outline) = q_outline.get_mut(selectable.1) {
                    outline.visible = true;
                }
            }
        }
        *select_box = SelectBox {
            scaled: None,
            unscaled: None,
        };
    }
}
