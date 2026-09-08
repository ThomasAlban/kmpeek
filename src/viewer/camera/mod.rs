use self::{fly::fly_cam_plugin, gizmo_2d::gizmo_2d_cam_plugin, orbit::orbit_cam_plugin, topdown::topdown_cam_plugin};
pub use self::{
    fly::{FlyCam, FlySettings},
    gizmo_2d::Gizmo2dCam,
    orbit::{OrbitCam, OrbitSettings},
    topdown::{TopDownCam, TopDownSettings},
};
use crate::ui::{settings::AppSettings, update_ui::UpdateUiSet, viewport::ViewportInfo};
use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, IntoStaticStr};

mod fly;
mod gizmo_2d;
mod orbit;
mod topdown;

pub fn camera_plugin(app: &mut App) {
    app.add_plugins((
        fly_cam_plugin,
        orbit_cam_plugin,
        topdown_cam_plugin,
        gizmo_2d_cam_plugin,
    ))
    .configure_sets(Update, UpdateCameraSet.before(UpdateUiSet))
    .add_message::<CameraModeChanged>()
    .add_systems(Startup, add_ambient_light)
    .add_systems(Update, (cursor_grab, update_active_camera));
}

#[derive(Component)]
pub struct EditorCamera;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UpdateCameraSet;

#[derive(PartialEq, Clone, Copy, Serialize, Deserialize, Default, Debug, IntoStaticStr, EnumString, Display)]
pub enum CameraMode {
    #[default]
    Fly,
    Orbit,
    #[strum(serialize = "Top Down")]
    TopDown,
}

#[derive(Message)]
pub struct CameraModeChanged(pub CameraMode);

#[derive(Default, Serialize, Deserialize)]
pub struct CameraSettings {
    pub mode: CameraMode,
    pub fly: FlySettings,
    pub orbit: OrbitSettings,
    pub top_down: TopDownSettings,
}

fn add_ambient_light(mut commands: Commands) {
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 1000.,
        affects_lightmapped_meshes: true,
    });
}

fn cursor_grab(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    // mut q_window: Query<&mut Window>,
    mut q_cursor_options: Query<&mut CursorOptions>,
    settings: Res<AppSettings>,
    viewport_info: Res<ViewportInfo>,
) {
    if !viewport_info.mouse_in_viewport {
        return;
    }
    // let mut window = q_window.single_mut().unwrap();
    let cursor: &mut CursorOptions = &mut q_cursor_options.single_mut().unwrap();

    if (settings.camera.mode == CameraMode::Fly
        && !mouse_buttons.pressed(settings.camera.fly.key_bindings.mouse_button))
        || (settings.camera.mode == CameraMode::Orbit
            && !mouse_buttons.pressed(settings.camera.orbit.key_bindings.mouse_button))
        || (settings.camera.mode == CameraMode::TopDown
            && !mouse_buttons.pressed(settings.camera.top_down.key_bindings.mouse_button))
    {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
        return;
    }
    // hide the cursor and lock its position
    cursor.visible = false;
    cursor.grab_mode = CursorGrabMode::Locked;
}

fn update_active_camera(
    mut q_fly_cam: Query<&mut Camera, (With<FlyCam>, Without<OrbitCam>, Without<TopDownCam>)>,
    mut q_orbit_cam: Query<&mut Camera, (With<OrbitCam>, Without<FlyCam>, Without<TopDownCam>)>,
    mut q_topdown_cam: Query<&mut Camera, (With<TopDownCam>, Without<FlyCam>, Without<OrbitCam>)>,
    mut ev_camera_mode_changed: MessageReader<CameraModeChanged>,
) {
    for ev in ev_camera_mode_changed.read() {
        let mut fly_cam = q_fly_cam.single_mut().unwrap();
        let mut orbit_cam = q_orbit_cam.single_mut().unwrap();
        let mut topdown_cam = q_topdown_cam.single_mut().unwrap();

        let active_states = match ev.0 {
            CameraMode::Fly => (true, false, false),
            CameraMode::Orbit => (false, true, false),
            CameraMode::TopDown => (false, false, true),
        };
        fly_cam.is_active = active_states.0;
        orbit_cam.is_active = active_states.1;
        topdown_cam.is_active = active_states.2;
    }
}
