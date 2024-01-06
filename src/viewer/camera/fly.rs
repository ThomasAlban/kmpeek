use crate::ui::{
    app_state::{AppSettings, AppState},
    viewport::{SetupViewportSet, ViewportImage},
};
use bevy::{
    input::mouse::MouseMotion,
    math::vec3,
    prelude::*,
    render::camera::RenderTarget,
    window::{CursorGrabMode, PrimaryWindow, RequestRedraw},
};
use serde::{Deserialize, Serialize};

use super::CameraMode;

pub struct FlyCamPlugin;
impl Plugin for FlyCamPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, camera_setup.after(SetupViewportSet))
            .add_systems(Update, (fly_cam_look, fly_cam_move));
    }
}

#[derive(Component)]
pub struct FlyCam;

#[derive(Serialize, Deserialize)]
pub struct FlySettings {
    pub start_pos: Vec3,
    pub look_sensitivity: f32,
    pub hold_mouse_to_move: bool,
    pub speed: f32,
    pub speed_boost: f32,
    pub key_bindings: FlyKeyBindings,
}
impl Default for FlySettings {
    fn default() -> Self {
        Self {
            start_pos: vec3(50000., 50000., 0.),
            look_sensitivity: 1.,
            hold_mouse_to_move: false,
            speed: 1.,
            speed_boost: 3.,
            key_bindings: FlyKeyBindings::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct FlyKeyBindings {
    pub move_forward: Vec<KeyCode>,
    pub move_backward: Vec<KeyCode>,
    pub move_left: Vec<KeyCode>,
    pub move_right: Vec<KeyCode>,
    pub move_ascend: Vec<KeyCode>,
    pub move_descend: Vec<KeyCode>,
    pub speed_boost: Vec<KeyCode>,
    pub mouse_button: MouseButton,
}

impl Default for FlyKeyBindings {
    fn default() -> Self {
        Self {
            move_forward: vec![KeyCode::W, KeyCode::Up],
            move_backward: vec![KeyCode::S, KeyCode::Down],
            move_left: vec![KeyCode::A, KeyCode::Left],
            move_right: vec![KeyCode::D, KeyCode::Right],
            move_ascend: vec![KeyCode::E, KeyCode::PageUp],
            move_descend: vec![KeyCode::Q, KeyCode::PageDown],
            speed_boost: vec![KeyCode::ShiftLeft, KeyCode::ShiftRight],
            mouse_button: MouseButton::Right,
        }
    }
}

fn camera_setup(mut commands: Commands, viewport: Res<ViewportImage>) {
    let fly_default = FlySettings::default();

    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                // render to the image
                target: RenderTarget::Image(viewport.handle.clone()),
                ..default()
            },
            transform: Transform::from_translation(fly_default.start_pos)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        FlyCam,
    ));
}

fn fly_cam_move(
    keys: Res<Input<KeyCode>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut fly_cam: Query<&mut Transform, With<FlyCam>>,
    app_state: Res<AppState>,
    mut ev_request_redraw: EventWriter<RequestRedraw>,
    settings: Res<AppSettings>,
) {
    if !app_state.mouse_in_viewport || settings.camera.mode != CameraMode::Fly {
        return;
    }
    // if we are pressing the control / cmd key, return
    if (!cfg!(target_os = "macos")
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)))
        || (cfg!(target_os = "macos")
            && (keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight)))
    {
        return;
    }

    let window = window.get_single().unwrap();
    // if we need to be holding the mouse to move but we aren't, return
    if settings.camera.fly.hold_mouse_to_move && window.cursor.grab_mode == CursorGrabMode::None {
        return;
    }

    let mut fly_cam_transform = fly_cam.get_single_mut().unwrap();

    let mut velocity = Vec3::ZERO;
    let local_z = fly_cam_transform.local_z();
    let forward = -Vec3::new(local_z.x, 0., local_z.z);
    let right = Vec3::new(local_z.z, 0., -local_z.x);

    let mut speed_boost = false;

    if keys.get_pressed().count() > 0 {
        ev_request_redraw.send(RequestRedraw);
    }

    for key in keys.get_pressed() {
        let key_bindings = &settings.camera.fly.key_bindings;
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
        velocity *= settings.camera.fly.speed_boost;
    }

    fly_cam_transform.translation += velocity * 100. * settings.camera.fly.speed;
}

fn fly_cam_look(
    window: Query<&Window, With<PrimaryWindow>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut fly_cam: Query<&mut Transform, With<FlyCam>>,
    app_state: Res<AppState>,
    settings: Res<AppSettings>,
) {
    if !app_state.mouse_in_viewport || settings.camera.mode != CameraMode::Fly {
        return;
    }

    let window = window.get_single().unwrap();
    let mut fly_cam_transform = fly_cam.get_single_mut().unwrap();

    for ev in mouse_motion.read() {
        let (mut yaw, mut pitch, _) = fly_cam_transform.rotation.to_euler(EulerRot::YXZ);
        match window.cursor.grab_mode {
            CursorGrabMode::None => (),
            _ => {
                // Using smallest of height or width ensures equal vertical and horizontal sensitivity
                let window_scale = window.height().min(window.width());
                pitch -=
                    (settings.camera.fly.look_sensitivity * 0.00012 * ev.delta.y * window_scale)
                        .to_radians();
                yaw -= (settings.camera.fly.look_sensitivity * 0.00012 * ev.delta.x * window_scale)
                    .to_radians();
            }
        }
        pitch = pitch.clamp(-1.54, 1.54);
        // order is important to prevent unintended roll
        fly_cam_transform.rotation =
            Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch);
    }
}
