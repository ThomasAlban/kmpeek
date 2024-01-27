use super::CameraMode;
use crate::ui::{
    settings::AppSettings,
    ui_state::MouseInViewport,
    update_ui::UpdateUiSet,
    viewport::{SetupViewportSet, ViewportImage},
};
use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    math::vec3,
    prelude::*,
    render::camera::RenderTarget,
    window::PrimaryWindow,
};
use serde::{Deserialize, Serialize};

pub struct OrbitCamPlugin;
impl Plugin for OrbitCamPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, camera_setup.after(SetupViewportSet))
            .add_systems(Update, orbit_cam.before(UpdateUiSet));
    }
}

#[derive(Component, Clone, Copy, PartialEq)]
pub struct OrbitCam {
    pub focus: Vec3,
    pub radius: f32,
    pub upside_down: bool,
}
impl Default for OrbitCam {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            radius: 10000.,
            upside_down: false,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct OrbitSettings {
    pub start_pos: Vec3,
    pub rotate_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub scroll_sensitivity: f32,
    pub key_bindings: OrbitKeyBindings,
}
impl Default for OrbitSettings {
    fn default() -> Self {
        Self {
            start_pos: vec3(50000., 50000., 0.),
            rotate_sensitivity: 1.,
            pan_sensitivity: 1.,
            scroll_sensitivity: 1.,
            key_bindings: OrbitKeyBindings::default(),
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct OrbitKeyBindings {
    pub mouse_button: MouseButton,
    pub pan: Vec<KeyCode>,
}
impl Default for OrbitKeyBindings {
    fn default() -> Self {
        Self {
            mouse_button: MouseButton::Right,
            pan: vec![KeyCode::ShiftLeft, KeyCode::ShiftRight],
        }
    }
}

fn camera_setup(mut commands: Commands, viewport: Res<ViewportImage>) {
    let orbit_default = OrbitSettings::default();

    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                // render to the image
                target: RenderTarget::Image(viewport.handle.clone()),
                is_active: false,
                ..default()
            },
            transform: Transform::from_translation(orbit_default.start_pos)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        OrbitCam {
            radius: OrbitSettings::default().start_pos.length(),
            ..default()
        },
        // RaycastSource::<KmpRaycastSet>::new(),
        // RaycastSource::<KclRaycastSet>::new(),
    ));
}

fn orbit_cam(
    q_window: Query<&mut Window, With<PrimaryWindow>>,
    mut ev_mouse_motion: EventReader<MouseMotion>,
    mut ev_mouse_scroll: EventReader<MouseWheel>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut q_orbit_cam: Query<(&mut OrbitCam, &mut Transform, &Projection)>,
    keys: Res<Input<KeyCode>>,
    settings: Res<AppSettings>,
    mouse_in_viewport: Res<MouseInViewport>,
) {
    if !mouse_in_viewport.0 || settings.camera.mode != CameraMode::Orbit {
        return;
    }

    let window = q_window.get_single().unwrap();

    let mut pan = Vec2::ZERO;
    let mut rotation_move = Vec2::ZERO;
    let mut scroll = 0.0;
    let mut orbit_button_changed = false;

    if mouse_buttons.pressed(settings.camera.orbit.key_bindings.mouse_button) {
        // check if the pan key is being pressed, and if so, pan rather than orbit
        let mut rotate = true;
        for key in keys.get_pressed() {
            if settings.camera.orbit.key_bindings.pan.contains(key) {
                rotate = false;
                break;
            }
        }
        if rotate {
            for ev in ev_mouse_motion.read() {
                rotation_move += ev.delta * settings.camera.orbit.rotate_sensitivity;
            }
        } else {
            for ev in ev_mouse_motion.read() {
                pan += ev.delta * settings.camera.orbit.pan_sensitivity;
            }
        }
    }

    for ev in ev_mouse_scroll.read() {
        scroll += ev.y;
    }

    if mouse_buttons.just_released(settings.camera.orbit.key_bindings.mouse_button)
        || mouse_buttons.just_pressed(settings.camera.orbit.key_bindings.mouse_button)
    {
        orbit_button_changed = true;
    }

    let (mut orbit_cam, mut transform, projection) = q_orbit_cam.single_mut();
    let mut transform_cp = *transform;
    let mut orbit_cam_cp = *orbit_cam;

    if orbit_button_changed {
        // only check for upside down when orbiting started or ended this frame
        // if the camera is "upside" down, panning horizontally would be inverted, so invert the input to make it correct
        let up = transform_cp.rotation * Vec3::Y;
        orbit_cam_cp.upside_down = up.y <= 0.0;
    }

    let window_size = Vec2::new(window.width(), window.height());

    let mut any = false;
    if rotation_move.length_squared() > 0.0 {
        any = true;
        let delta_x = {
            let delta = rotation_move.x / window_size.x * std::f32::consts::PI * 2.0;
            if orbit_cam_cp.upside_down {
                -delta
            } else {
                delta
            }
        };
        let delta_y = rotation_move.y / window_size.y * std::f32::consts::PI;

        let yaw = Quat::from_rotation_y(-delta_x);
        let pitch = Quat::from_rotation_x(-delta_y);
        transform_cp.rotation = yaw * transform_cp.rotation; // rotate around global y axis
        transform_cp.rotation *= pitch; // rotate around local x axis
    } else if pan.length_squared() > 0.0 {
        any = true;
        // make panning distance independent of resolution and FOV
        if let Projection::Perspective(projection) = projection {
            pan *=
                Vec2::new(projection.fov * projection.aspect_ratio, projection.fov) / window_size;
        }
        // translate by local axes
        let right = transform_cp.rotation * Vec3::X * -pan.x;
        let up = transform_cp.rotation * Vec3::Y * pan.y;
        // make panning proportional to distance away from focus point
        let translation = (right + up) * orbit_cam_cp.radius;
        orbit_cam_cp.focus += translation;
    } else if scroll.abs() > 0.0 {
        any = true;
        orbit_cam_cp.radius -=
            scroll * orbit_cam_cp.radius * 0.002 * settings.camera.orbit.scroll_sensitivity;
        // dont allow zoom to reach zero or you get stuck
        orbit_cam_cp.radius = orbit_cam_cp.radius.clamp(1., 500000.);
    }

    if any {
        // emulating parent/child to make the yaw/y-axis rotation behave like a turntable
        // parent = x and y rotation
        // child = z-offset
        let rot_matrix = Mat3::from_quat(transform_cp.rotation);
        transform_cp.translation =
            orbit_cam_cp.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, orbit_cam_cp.radius));
    }

    transform.set_if_neq(transform_cp);
    orbit_cam.set_if_neq(orbit_cam_cp);

    // consume any remaining events, so they don't pile up if we don't need them
    // (and also to avoid Bevy warning us about not checking events every frame update)
    ev_mouse_motion.clear();
}
