use super::{components::*, resources::*};

use bevy::{
    ecs::event::Events,
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_mod_picking::prelude::*;

pub fn camera_setup(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle::default(),
        FlyCam,
        RaycastPickCamera::default(),
    ));
}

// handles keyboard input and movement
pub fn camera_move(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    settings: Res<MovementSettings>,
    key_bindings: Res<KeyBindings>,
    mut query: Query<(&FlyCam, &mut Transform)>,
) {
    if let Ok(window) = primary_window.get_single() {
        for (_camera, mut transform) in query.iter_mut() {
            let mut velocity = Vec3::ZERO;
            let local_z = transform.local_z();
            let forward = -Vec3::new(local_z.x, 0., local_z.z);
            let right = Vec3::new(local_z.z, 0., -local_z.x);

            if window.cursor.grab_mode == CursorGrabMode::None {
                return;
            }
            let mut speed_boost = false;
            for key in keys.get_pressed() {
                if key_bindings.move_forward.contains(key) {
                    velocity += forward;
                } else if key_bindings.move_backward.contains(key) {
                    velocity -= forward;
                } else if key_bindings.move_left.contains(key) {
                    velocity -= right;
                } else if key_bindings.move_right.contains(key) {
                    velocity += right;
                } else if key_bindings.move_ascend.contains(key) {
                    velocity += Vec3::Y;
                } else if key_bindings.move_descend.contains(key) {
                    velocity -= Vec3::Y;
                } else if key_bindings.speed_boost.contains(key) {
                    speed_boost = true;
                }
            }

            if speed_boost {
                velocity *= settings.speed_boost;
            }

            transform.translation += velocity * time.delta_seconds() * settings.speed;
        }
    } else {
        warn!("Primary window not found for camera controller");
    }
}

// handles looking around if cursor is locked
pub fn camera_look(
    settings: Res<MovementSettings>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<InputState>,
    motion: Res<Events<MouseMotion>>,
    mut query: Query<&mut Transform, With<FlyCam>>,
) {
    if let Ok(window) = primary_window.get_single() {
        for mut transform in query.iter_mut() {
            for ev in state.reader_motion.iter(&motion) {
                let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
                match window.cursor.grab_mode {
                    CursorGrabMode::None => (),
                    _ => {
                        // Using smallest of height or width ensures equal vertical and horizontal sensitivity
                        let window_scale = window.height().min(window.width());
                        pitch -= (settings.sensitivity * ev.delta.y * window_scale).to_radians();
                        yaw -= (settings.sensitivity * ev.delta.x * window_scale).to_radians();
                    }
                }

                pitch = pitch.clamp(-1.54, 1.54);

                // order is important to prevent unintended roll
                transform.rotation =
                    Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch);
            }
        }
    } else {
        warn!("Primary window not found for camera controller");
    }
}

pub fn cursor_grab(
    mouse_buttons: Res<Input<MouseButton>>,
    key_bindings: Res<KeyBindings>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok(mut window) = primary_window.get_single_mut() {
        // if the right mouse is not pressed then unlock the cursor and return
        if !mouse_buttons.pressed(key_bindings.mouse_button) {
            window.cursor.visible = true;
            window.cursor.grab_mode = CursorGrabMode::None;
            return;
        }
        // hide the cursor and lock its position
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
    } else {
        warn!("Primary window not found for `cursor_grab`!");
    }
}
