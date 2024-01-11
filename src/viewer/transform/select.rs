use crate::ui::ui_state::{MouseInViewport, ViewportRect};
use crate::viewer::kmp::components::KmpSection;
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_mod_outline::*;
use bevy_mod_raycast::prelude::*;

use super::gizmo::GizmoOptions;

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
    window: Query<&Window, With<PrimaryWindow>>,
    keys: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut raycast: Raycast,
    kmp_section_query: Query<&KmpSection>,
    mut commands: Commands,
    gizmo: Res<GizmoOptions>,
    mut outline: Query<&mut OutlineVolume>,
    selected_query: Query<Entity, With<Selected>>,
    visibility_q: Query<&Visibility>,
) {
    if !mouse_in_viewport.0 || !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let window = window.get_single().unwrap();
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };
    if gizmo.last_result.is_some() {
        return;
    }
    let shift_key_down = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // get the active camera
    let cam = camera_query
        .iter()
        .filter(|cam| cam.0.is_active)
        .collect::<Vec<(&Camera, &GlobalTransform)>>()[0];

    let scaled_mouse_pos = scale_viewport_pos(mouse_pos, window, viewport_rect.0);
    // send out a ray
    let intersections = cast_ray_from_cam(cam, scaled_mouse_pos, &mut raycast, |e| {
        let visibility = visibility_q.get(e).unwrap();
        kmp_section_query.contains(e) && visibility == Visibility::Visible
    });
    let intersection = intersections.first();

    // deselect everything if we already have something selected but don't have the shift key down
    if intersection.is_some() && !shift_key_down {
        for selected in selected_query.iter() {
            commands.entity(selected).remove::<Selected>();
            // remove the outline
            if let Ok(mut outline) = outline.get_mut(selected) {
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
        if let Ok(mut outline) = outline.get_mut(*to_select) {
            outline.visible = true;
        }
    } else if !shift_key_down {
        // if we just randomly clicked on nothing then deselect everything
        for selected in selected_query.iter() {
            commands.entity(selected).remove::<Selected>();
            // remove the outline
            if let Ok(mut outline) = outline.get_mut(selected) {
                outline.visible = false;
            }
        }
    }
}

pub fn deselect_on_mode_change(
    mut commands: Commands,
    selected_q: Query<Entity, With<Selected>>,
    mut outline: Query<&mut OutlineVolume>,
) {
    for selected in selected_q.iter() {
        commands.entity(selected).remove::<Selected>();
        if let Ok(mut outline) = outline.get_mut(selected) {
            outline.visible = false;
        }
    }
}
