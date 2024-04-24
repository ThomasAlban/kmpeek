use super::{
    select::{SelectSet, Selected},
    EditMode,
};
use crate::{
    ui::viewport::ViewportInfo,
    util::{get_ray_from_cam, ui_viewport_to_ndc, RaycastFromCam},
    viewer::{camera::Gizmo2dCam, kcl_model::KCLModelSection},
};
use bevy::{prelude::*, utils::HashMap};
use bevy_mod_raycast::prelude::*;

pub struct KclSnapPlugin;
impl Plugin for KclSnapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, snap_to_kcl.after(SelectSet));
    }
}

fn snap_to_kcl(
    mut q_selected: Query<(&mut Transform, Entity), With<Selected>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), Without<Gizmo2dCam>>,
    q_window: Query<&Window>,
    mut raycast: Raycast,
    q_kcl: Query<(), With<KCLModelSection>>,
    edit_mode: Res<EditMode>,
    viewport_info: Res<ViewportInfo>,

    mut initial_offset_ndc: Local<Vec2>,
    mut initial_intersection_point: Local<Vec3>,
    mut initial_mouse_pos: Local<Vec2>,
    mut position_differences: Local<HashMap<Entity, Vec3>>,
) {
    if *edit_mode != EditMode::Tweak || !viewport_info.mouse_in_viewport {
        return;
    }

    // only snap to kcl if we are currently pressing left click
    if !mouse_buttons.pressed(MouseButton::Left) {
        // clear the position differences
        if mouse_buttons.just_released(MouseButton::Left) {
            position_differences.clear();
        }
        return;
    };
    let window = q_window.get_single().unwrap();
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };
    // get the active camera
    let cam = q_camera.iter().find(|cam| cam.0.is_active).unwrap();

    let mouse_pos_ndc = ui_viewport_to_ndc(mouse_pos, viewport_info.viewport_rect);

    if mouse_buttons.just_pressed(MouseButton::Left) {
        *initial_mouse_pos = mouse_pos;
        // get the transform of the thing the mouse has just clicked on
        let ray = RaycastFromCam::new(cam, mouse_pos_ndc, &mut raycast)
            .filter(&|e| q_selected.contains(e))
            .cast();
        let Some((mouse_over, _)) = ray.first() else {
            return;
        };
        let (main_point_transform, _) = q_selected.get(*mouse_over).unwrap();

        // get the position of the entity we are going to start dragging
        let pos = main_point_transform.translation;
        // translate this position into screenspace coords
        let pos_ndc = cam.0.world_to_ndc(cam.1, pos).unwrap().xy();

        // calculate the offset between where we have clicked and where the entity is on the screen
        *initial_offset_ndc = pos_ndc - mouse_pos_ndc;

        // set the distance between the camera and this entity's transform, saving it for later in case we drag outside of kcl
        *initial_intersection_point = main_point_transform.translation;

        // go through and set the position differences of each selected entity relative to this one
        for selected in q_selected.iter() {
            let position_difference = selected.0.translation - main_point_transform.translation;
            position_differences.insert(selected.1, position_difference);
        }
        // return since we only want to update the positions of the entities if we move the mouse
        return;
    }
    // if the mouse hasn't moved we don't want to update the positions of the entities
    if mouse_pos == *initial_mouse_pos {
        return;
    }

    // send out a ray from the mouse position + the offset
    // so that when an entity is initially clicked, it's transform doesn't change even though they weren't perfectly accurate with the click
    let intersections = RaycastFromCam::new(cam, mouse_pos_ndc + *initial_offset_ndc, &mut raycast)
        .filter(&|e| q_kcl.contains(e))
        .cast();

    if let Some(intersection) = intersections.first() {
        // if there is an intersection with the kcl, snap to the kcl
        for mut selected in q_selected.iter_mut() {
            let Some(position_difference) = position_differences.get(&selected.1) else {
                return;
            };
            selected.0.translation = intersection.1.position() + *position_difference;
        }
    } else {
        // if there is no intersection with the kcl, move the point in the camera plane based on where we started dragging the point
        let ray = get_ray_from_cam(cam, mouse_pos_ndc + *initial_offset_ndc).unwrap();
        for mut selected in q_selected.iter_mut() {
            let Some(position_difference) = position_differences.get(&selected.1) else {
                continue;
            };
            let camera_plane = Plane3d::new((-*initial_intersection_point + cam.1.translation()).normalize());
            let camera_plane_origin = *initial_intersection_point;
            if let Some(intersection_distance) = ray.intersect_plane(camera_plane_origin, camera_plane) {
                let pos_on_plane = ray.get_point(intersection_distance);
                selected.0.translation = pos_on_plane + *position_difference;
            }
        }
    }
}
