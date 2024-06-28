use super::{
    create_delete::JustCreatedPoint,
    select::{SelectSet, Selected},
    EditMode,
};
use crate::{
    ui::viewport::ViewportInfo,
    util::{get_ray_from_cam, ui_viewport_to_ndc, RaycastFromCam},
    viewer::{camera::Gizmo2dCam, kcl_model::KCLModelSection, kmp::checkpoints::CheckpointHeight},
};
use bevy::{prelude::*, utils::HashMap};
use bevy_mod_raycast::prelude::*;

#[derive(Component)]
pub struct Tweakable(pub SnapTo);

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SnapTo {
    Kcl,
    CheckpointPlane,
}

#[derive(Component)]
pub struct SnapToKcl;

#[derive(Component)]
pub struct SnapToCheckpointPlane;

pub fn tweak_plugin(app: &mut App) {
    app.add_systems(Update, tweak_interaction.after(SelectSet));
}

#[derive(Resource, Clone, Debug)]
pub struct TweakInteraction {
    tweak_type: SnapTo,
    /// The mouse position when we started the interaction.
    initial_mouse_pos: Vec2,
    /// The ndc offset between the point being dragged and the mouse pos.
    offset_ndc: Vec2,
    /// The distance in 3d coords to the main point being dragged.
    position_differences: HashMap<Entity, Vec3>,
    /// The initial 3d position of the main point being dragged, so if we drag
    /// outside a snap zone we can move in the camera plane at the correct distance.
    initial_interaction_point: Vec3,
}

pub fn tweak_interaction(
    mut tweak_interaction: Local<Option<TweakInteraction>>,
    mut q_selected: Query<(Entity, &mut Transform, &Tweakable), With<Selected>>,
    edit_mode: Res<EditMode>,
    viewport_info: Res<ViewportInfo>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    q_window: Query<&Window>,
    q_camera: Query<(&Camera, &GlobalTransform), Without<Gizmo2dCam>>,
    mut raycast: Raycast,
    checkpoint_height: Res<CheckpointHeight>,
    q_kcl: Query<(), With<KCLModelSection>>,
    mut ev_just_created_point: EventReader<JustCreatedPoint>,
) {
    if *edit_mode != EditMode::Tweak || !viewport_info.mouse_in_viewport || q_selected.is_empty() {
        return;
    }
    if !mouse_buttons.pressed(MouseButton::Left) {
        // clear the interaction
        if mouse_buttons.just_released(MouseButton::Left) {
            *tweak_interaction = None;
        }
        return;
    };

    let window = q_window.single();
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };
    // get the active camera
    let cam = q_camera.iter().find(|cam| cam.0.is_active).unwrap();

    let mouse_pos_ndc = ui_viewport_to_ndc(mouse_pos, viewport_info.viewport_rect);

    if mouse_buttons.just_pressed(MouseButton::Left) {
        // get the transform of the thing the mouse has just clicked on
        let ray = RaycastFromCam::new(cam, mouse_pos_ndc, &mut raycast)
            .filter(&|e| q_selected.contains(e))
            .cast();

        let mouse_over_entity = match ray.first() {
            Some(e) => e.0,
            // if there is no intersection, then deal with the possibility that we just created a checkpoint,
            // so want to interact with the right hand node of the newly created cp
            None => {
                let Some(e) = ev_just_created_point.read().next().map(|x| x.0) else {
                    return;
                };
                e
            }
        };

        // if we got this far it means we just clicked on a tweakable point
        let (_, mouse_over_transform, _) = q_selected.get(mouse_over_entity).unwrap();

        // get the position of the entity we are going to start dragging
        let pos = mouse_over_transform.translation;
        // translate this position into screenspace coords
        let pos_ndc = cam.0.world_to_ndc(cam.1, pos).unwrap().xy();

        let mut position_differences = HashMap::new();

        for selected in q_selected.iter() {
            // go through and set the position differences of each selected entity relative to this one
            let position_difference = selected.1.translation - pos;
            position_differences.insert(selected.0, position_difference);
        }

        // we can't allow tweak interactions where they are not all the same type as this would lead to weird behaviour
        let tweak_type = q_selected.iter().next().unwrap().2 .0;
        if q_selected.iter().any(|x| x.2 .0 != tweak_type) {
            return;
        }

        *tweak_interaction = Some(TweakInteraction {
            tweak_type,
            initial_mouse_pos: mouse_pos,
            offset_ndc: pos_ndc - mouse_pos_ndc,
            initial_interaction_point: pos,
            position_differences,
        });

        // return since we only want to update the positions of the entities if we move the mouse
        return;
    }
    let Some(tweak_interaction) = tweak_interaction.clone() else {
        return;
    };
    // if the mouse hasn't moved we don't want to update the positions of the entities
    if mouse_pos == tweak_interaction.initial_mouse_pos {
        return;
    }

    // send out a ray from the mouse position + the offset
    // so that when an entity is initially clicked, it's transform doesn't change even though they weren't perfectly accurate with the click
    let Some(cam_ray) = get_ray_from_cam(cam, mouse_pos_ndc + tweak_interaction.offset_ndc) else {
        return;
    };

    let snap_pos = match tweak_interaction.tweak_type {
        SnapTo::Kcl => {
            let intersections =
                raycast.cast_ray(cam_ray, &RaycastSettings::default().with_filter(&|e| q_kcl.contains(e)));
            intersections.first().map(|x| x.1.position())
        }
        SnapTo::CheckpointPlane => {
            let dist = cam_ray.intersect_plane(Vec3::Y * checkpoint_height.0, Plane3d::default());
            dist.map(|x| cam_ray.get_point(x))
        }
    };

    for mut selected in q_selected.iter_mut() {
        let Some(position_difference) = tweak_interaction.position_differences.get(&selected.0) else {
            continue;
        };

        if let Some(snap_pos) = snap_pos {
            // snap if possible
            selected.1.translation = snap_pos + *position_difference;
        } else {
            // if not possible to snap, move the point in the camera plane based on where we started dragging the point
            let camera_plane =
                Plane3d::new((-tweak_interaction.initial_interaction_point + cam.1.translation()).normalize());
            let camera_plane_origin = tweak_interaction.initial_interaction_point;

            if let Some(dist) = cam_ray.intersect_plane(camera_plane_origin, camera_plane) {
                let pos_on_plane = cam_ray.get_point(dist);
                selected.1.translation = pos_on_plane + *position_difference;
            }
        }
    }
}
