use super::{components::*, resources::*};

use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_mod_picking::prelude::*;

pub fn camera_setup(mut commands: Commands) {
    let start_position = Vec3::new(50000., 50000., 0.);
    let transform = Transform::from_translation(start_position).looking_at(Vec3::ZERO, Vec3::Y);
    commands.spawn((
        Camera3dBundle {
            transform,
            ..default()
        },
        FlyCam,
        RaycastPickCamera::default(),
    ));
    commands.spawn((
        Camera3dBundle {
            transform,
            camera: Camera {
                is_active: false,
                ..default()
            },
            ..default()
        },
        OrbitCam {
            radius: start_position.length(),
            ..default()
        },
        RaycastPickCamera::default(),
    ));
}

pub fn cursor_grab(
    mouse_buttons: Res<Input<MouseButton>>,
    settings: Res<CameraSettings>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok(mut window) = primary_window.get_single_mut() {
        if (settings.mode == CameraMode::Fly
            && !mouse_buttons.pressed(settings.fly.key_bindings.mouse_button))
            || (settings.mode == CameraMode::Orbit
                && !mouse_buttons.pressed(settings.orbit.key_bindings.mouse_button))
        {
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

pub fn fly_cam_move(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    settings: Res<CameraSettings>,
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
                let key_bindings = &settings.fly.key_bindings;
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
                velocity *= settings.fly.speed_boost;
            }

            transform.translation += velocity * time.delta_seconds() * settings.fly.speed;
        }
    } else {
        warn!("Primary window not found for camera controller");
    }
}
pub fn fly_cam_look(
    settings: Res<CameraSettings>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut query: Query<&mut Transform, With<FlyCam>>,
) {
    if let Ok(window) = primary_window.get_single() {
        for mut transform in query.iter_mut() {
            for ev in mouse_motion.iter() {
                let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
                match window.cursor.grab_mode {
                    CursorGrabMode::None => (),
                    _ => {
                        // Using smallest of height or width ensures equal vertical and horizontal sensitivity
                        let window_scale = window.height().min(window.width());
                        pitch -=
                            (settings.fly.sensitivity * ev.delta.y * window_scale).to_radians();
                        yaw -= (settings.fly.sensitivity * ev.delta.x * window_scale).to_radians();
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

pub fn orbit_cam(
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_scroll: EventReader<MouseWheel>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut query: Query<(&mut OrbitCam, &mut Transform, &Projection)>,
    settings: Res<CameraSettings>,
    keys: Res<Input<KeyCode>>,
) {
    if settings.mode != CameraMode::Orbit {
        return;
    }

    let window = primary_window.get_single();
    if window.is_err() {
        warn!("Primary window not found for camera controller");
        return;
    }
    let window = window.unwrap();

    let mut pan = Vec2::ZERO;
    let mut rotation_move = Vec2::ZERO;
    let mut scroll = 0.0;
    let mut orbit_button_changed = false;

    if mouse_buttons.pressed(settings.orbit.key_bindings.mouse_button) {
        // check if the pan key is being pressed, and if so, pan rather than orbit
        let mut rotate = true;
        for key in keys.get_pressed() {
            if settings.orbit.key_bindings.pan.contains(key) {
                rotate = false;
                break;
            }
        }
        if rotate {
            for ev in mouse_motion.iter() {
                rotation_move += ev.delta;
            }
        } else {
            for ev in mouse_motion.iter() {
                pan += ev.delta;
            }
        }
    }

    for ev in mouse_scroll.iter() {
        scroll += ev.y;
    }

    if mouse_buttons.just_released(settings.orbit.key_bindings.mouse_button)
        || mouse_buttons.just_pressed(settings.orbit.key_bindings.mouse_button)
    {
        orbit_button_changed = true;
    }

    for (mut pan_orbit, mut transform, projection) in query.iter_mut() {
        if orbit_button_changed {
            // only check for upside down when orbiting started or ended this frame
            // if the camera is "upside" down, panning horizontally would be inverted, so invert the input to make it correct
            let up = transform.rotation * Vec3::Y;
            pan_orbit.upside_down = up.y <= 0.0;
        }

        let window_size = Vec2::new(window.width(), window.height());

        let mut any = false;
        if rotation_move.length_squared() > 0.0 {
            any = true;
            let delta_x = {
                let delta = rotation_move.x / window_size.x * std::f32::consts::PI * 2.0;
                if pan_orbit.upside_down {
                    -delta
                } else {
                    delta
                }
            };
            let delta_y = rotation_move.y / window_size.y * std::f32::consts::PI;

            let yaw = Quat::from_rotation_y(-delta_x);
            let pitch = Quat::from_rotation_x(-delta_y);
            transform.rotation = yaw * transform.rotation; // rotate around global y axis
            transform.rotation *= pitch; // rotate around local x axis
        } else if pan.length_squared() > 0.0 {
            any = true;
            // make panning distance independent of resolution and FOV
            if let Projection::Perspective(projection) = projection {
                pan *= Vec2::new(projection.fov * projection.aspect_ratio, projection.fov)
                    / window_size;
            }
            // translate by local axes
            let right = transform.rotation * Vec3::X * -pan.x;
            let up = transform.rotation * Vec3::Y * pan.y;
            // make panning proportional to distance away from focus point
            let translation = (right + up) * pan_orbit.radius;
            pan_orbit.focus += translation;
        } else if scroll.abs() > 0.0 {
            any = true;
            // scroll currently not working
            pan_orbit.radius -= scroll * pan_orbit.radius * 0.002;
            // dont allow zoom to reach zero or you get stuck
            pan_orbit.radius = f32::max(pan_orbit.radius, 0.05);
        }

        if any {
            // emulating parent/child to make the yaw/y-axis rotation behave like a turntable
            // parent = x and y rotation
            // child = z-offset
            let rot_matrix = Mat3::from_quat(transform.rotation);
            transform.translation =
                pan_orbit.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, pan_orbit.radius));
        }
    }
    // consume any remaining events, so they don't pile up if we don't need them
    // (and also to avoid Bevy warning us about not checking events every frame update)
    mouse_motion.clear();
}
