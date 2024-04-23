use self::{fly::FlyCamPlugin, gizmo_2d::Gizmo2dCamPlugin, orbit::OrbitCamPlugin, topdown::TopDownCamPlugin};
pub use self::{
    fly::{FlyCam, FlySettings},
    gizmo_2d::Gizmo2dCam,
    orbit::{OrbitCam, OrbitSettings},
    topdown::{TopDownCam, TopDownSettings},
};
use crate::ui::{settings::AppSettings, ui_state::MouseInViewport, update_ui::UpdateUiSet};
use bevy::{prelude::*, window::CursorGrabMode};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, IntoStaticStr};

mod fly;
mod gizmo_2d;
mod orbit;
mod topdown;

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((FlyCamPlugin, OrbitCamPlugin, TopDownCamPlugin, Gizmo2dCamPlugin))
            .configure_sets(Update, UpdateCameraSet.before(UpdateUiSet))
            .add_event::<CameraModeChanged>()
            .add_systems(Startup, add_ambient_light)
            .add_systems(Update, (cursor_grab, update_active_camera));
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UpdateCameraSet;

#[derive(PartialEq, Clone, Copy, Serialize, Deserialize, Debug, IntoStaticStr, EnumString, Display)]
pub enum CameraMode {
    Fly,
    Orbit,
    #[strum(serialize = "Top Down")]
    TopDown,
}
impl Default for CameraMode {
    fn default() -> Self {
        Self::Fly
    }
}

#[derive(Event)]
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
    });
}

fn cursor_grab(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut q_window: Query<&mut Window>,
    settings: Res<AppSettings>,
    mouse_in_viewport: Res<MouseInViewport>,
) {
    if !mouse_in_viewport.0 {
        return;
    }
    let mut window = q_window.get_single_mut().unwrap();

    if (settings.camera.mode == CameraMode::Fly
        && !mouse_buttons.pressed(settings.camera.fly.key_bindings.mouse_button))
        || (settings.camera.mode == CameraMode::Orbit
            && !mouse_buttons.pressed(settings.camera.orbit.key_bindings.mouse_button))
        || (settings.camera.mode == CameraMode::TopDown
            && !mouse_buttons.pressed(settings.camera.top_down.key_bindings.mouse_button))
    {
        window.cursor.visible = true;
        window.cursor.grab_mode = CursorGrabMode::None;
        return;
    }
    // hide the cursor and lock its position
    window.cursor.visible = false;
    window.cursor.grab_mode = CursorGrabMode::Locked;
}

fn update_active_camera(
    mut q_fly_cam: Query<(Entity, &mut Camera), (With<FlyCam>, Without<OrbitCam>, Without<TopDownCam>)>,
    mut q_orbit_cam: Query<(Entity, &mut Camera), (With<OrbitCam>, Without<FlyCam>, Without<TopDownCam>)>,
    mut q_topdown_cam: Query<(Entity, &mut Camera), (With<TopDownCam>, Without<FlyCam>, Without<OrbitCam>)>,
    mut ev_camera_mode_changed: EventReader<CameraModeChanged>,
) {
    for ev in ev_camera_mode_changed.read() {
        let mut fly_cam = q_fly_cam.get_single_mut().unwrap();
        let mut orbit_cam = q_orbit_cam.get_single_mut().unwrap();
        let mut topdown_cam = q_topdown_cam.get_single_mut().unwrap();

        match ev.0 {
            CameraMode::Fly => {
                fly_cam.1.is_active = true;
                orbit_cam.1.is_active = false;
                topdown_cam.1.is_active = false;
            }
            CameraMode::Orbit => {
                fly_cam.1.is_active = false;
                orbit_cam.1.is_active = true;
                topdown_cam.1.is_active = false;
            }
            CameraMode::TopDown => {
                fly_cam.1.is_active = false;
                orbit_cam.1.is_active = false;
                topdown_cam.1.is_active = true;
            }
        }
    }
}
