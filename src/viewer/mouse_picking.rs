use bevy::{prelude::*, window::PrimaryWindow};
use bevy_mod_outline::*;
use bevy_mod_raycast::prelude::*;

use crate::ui::app_state::AppState;

use super::{kcl_model::KCLModelSection, kmp::components::KmpSection};

pub struct MousePickingPlugin;
impl Plugin for MousePickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DefaultRaycastingPlugin, OutlinePlugin))
            .add_systems(
                Update,
                (
                    select_entity,
                    // make sure we apply all the selections before updating outlines and positions
                    apply_deferred,
                    update_outlines,
                    snap_to_kcl,
                )
                    .chain(),
            );
    }
}

fn scale_viewport_pos(viewport_pos: Vec2, window: &Window, viewport_rect: Rect) -> Vec2 {
    // make (0,0) be the top left corner of the viewport
    let mut scaled_viewport_pos = viewport_pos - viewport_rect.min;
    scaled_viewport_pos = scaled_viewport_pos.clamp(Vec2::ZERO, viewport_rect.max);
    scaled_viewport_pos *= window.scale_factor() as f32;
    scaled_viewport_pos
}

// fn cast_ray

fn cast_ray_from_cam(
    cam: (&Camera, &GlobalTransform),
    scaled_viewport_pos: Vec2,
    raycast: &mut Raycast,
    filter: impl Fn(Entity) -> bool,
) -> Vec<(Entity, IntersectionData)> {
    let ray = cam
        .0
        .viewport_to_world(cam.1, scaled_viewport_pos)
        .map(Ray3d::from);

    let raycast_result = ray
        // if there's a ray, cast the ray and make a vector out of the result
        .map(|ray| {
            raycast
                .cast_ray(ray, &RaycastSettings::default().with_filter(&filter))
                .to_vec()
        })
        // if there's no result return an empty vector
        .map_or(Vec::new(), |res| res);

    raycast_result
}

#[derive(Component)]
pub struct Selected;

fn select_entity(
    app_state: Res<AppState>,
    window: Query<&Window, With<PrimaryWindow>>,
    keys: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut raycast: Raycast,
    selected_query: Query<Entity, With<Selected>>,
    kmp_section_query: Query<&KmpSection>,
    mut commands: Commands,
) {
    if !app_state.mouse_in_viewport {
        return;
    }
    let window = window.get_single().unwrap();
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };
    let shift_key_down = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // get the active camera
    let cam = camera_query
        .iter()
        .filter(|cam| cam.0.is_active)
        .collect::<Vec<(&Camera, &GlobalTransform)>>()[0];

    if mouse_buttons.just_pressed(MouseButton::Left) {
        let scaled_mouse_pos = scale_viewport_pos(mouse_pos, window, app_state.viewport_rect);
        // send out a ray
        let intersections = cast_ray_from_cam(cam, scaled_mouse_pos, &mut raycast, |e| {
            kmp_section_query.contains(e)
        });
        let intersection = intersections.first();

        // deselect everything if we already have something selected but don't have the shift key down
        if intersection.is_some() && !shift_key_down {
            for selected in selected_query.iter() {
                commands.entity(selected).remove::<Selected>();
            }
        }
        // select the entity
        if let Some(intersection) = intersection {
            commands.entity(intersection.0).insert(Selected);
        } else if !shift_key_down {
            // if we just randomly clicked on nothing then deselect everything
            for selected in selected_query.iter() {
                commands.entity(selected).remove::<Selected>();
            }
        }
    }
}

fn snap_to_kcl(
    mut selected: Query<&mut Transform, With<Selected>>,
    mouse_buttons: Res<Input<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    window: Query<&Window, With<PrimaryWindow>>,
    app_state: Res<AppState>,
    mut raycast: Raycast,
    kcl_section_query: Query<&KCLModelSection>,
    mut offset: Local<Vec2>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) {
        return;
    }
    let Some(mut selected_transform) = selected.iter_mut().next() else {
        return;
    };
    let window = window.get_single().unwrap();
    let Some(mouse_pos) = window.cursor_position() else {
        return;
    };

    // get the active camera
    let cam = camera_query
        .iter()
        .filter(|cam| cam.0.is_active)
        .collect::<Vec<(&Camera, &GlobalTransform)>>()[0];

    let scaled_mouse_pos = scale_viewport_pos(mouse_pos, window, app_state.viewport_rect);

    if mouse_buttons.just_pressed(MouseButton::Left) {
        // get the position of the selected entity
        let pos = selected_transform.translation;
        // translate this position into screenspace coords
        let pos_screenspace = cam.0.world_to_viewport(cam.1, pos).unwrap();
        // calculate the offset between where we have clicked and where the entity is on the screen
        *offset = pos_screenspace - scaled_mouse_pos;
    }

    // send out a ray from the mouse position + the offset
    // so that when an entity is initially clicked, it's transform doesn't change even though they weren't perfectly accurate with the click
    let intersections = cast_ray_from_cam(cam, scaled_mouse_pos + *offset, &mut raycast, |e| {
        kcl_section_query.contains(e)
    });

    if let Some(intersection) = intersections.first() {
        let adjusted_pos = intersection.1.position();
        selected_transform.translation = adjusted_pos;
    }
}

fn update_outlines(mut query: Query<(&mut OutlineVolume, Option<&Selected>)>) {
    // set the outline to visible if the entity is selected, otherwise set it to not visible
    for mut selectable in query.iter_mut() {
        selectable.0.visible = selectable.1.is_some();
    }
}
