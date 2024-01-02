use self::{fly::FlyCamPlugin, orbit::OrbitCamPlugin, topdown::TopDownCamPlugin};
pub use self::{
    fly::{FlyCam, FlySettings},
    orbit::{OrbitCam, OrbitSettings},
    topdown::{TopDownCam, TopDownSettings},
};
use crate::ui::app_state::{AppSettings, AppState};
use bevy::{
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use serde::{Deserialize, Serialize};

mod fly;
mod orbit;
mod topdown;

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((FlyCamPlugin, OrbitCamPlugin, TopDownCamPlugin))
            .add_event::<CameraModeChanged>()
            .add_systems(Startup, add_ambient_light)
            .add_systems(Update, (cursor_grab, update_active_camera));
    }
}

#[derive(PartialEq, Clone, Copy, Serialize, Deserialize, Debug)]
pub enum CameraMode {
    Fly,
    Orbit,
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
        brightness: 0.5,
    });
}

fn cursor_grab(
    mouse_buttons: Res<Input<MouseButton>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    app_state: Res<AppState>,
    settings: Res<AppSettings>,
) {
    if !app_state.mouse_in_viewport {
        return;
    }
    let mut window = window.get_single_mut().unwrap();

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
    mut fly_cam: Query<
        (Entity, &mut Camera),
        (With<FlyCam>, Without<OrbitCam>, Without<TopDownCam>),
    >,
    mut orbit_cam: Query<
        (Entity, &mut Camera),
        (With<OrbitCam>, Without<FlyCam>, Without<TopDownCam>),
    >,
    mut topdown_cam: Query<
        (Entity, &mut Camera),
        (With<TopDownCam>, Without<FlyCam>, Without<OrbitCam>),
    >,
    // mut commands: Commands,
    mut ev_camera_mode_changed: EventReader<CameraModeChanged>,
) {
    for ev in ev_camera_mode_changed.read() {
        let mut fly_cam = fly_cam.get_single_mut().unwrap();
        let mut orbit_cam = orbit_cam.get_single_mut().unwrap();
        let mut topdown_cam = topdown_cam.get_single_mut().unwrap();

        match ev.0 {
            CameraMode::Fly => {
                // commands
                //     .entity(fly_cam.0)
                //     .insert(RaycastSource::<KmpRaycastSet>::new())
                //     .insert(RaycastSource::<KclRaycastSet>::new());
                fly_cam.1.is_active = true;
                // commands
                //     .entity(orbit_cam.0)
                //     .remove::<RaycastSource<KmpRaycastSet>>()
                //     .remove::<RaycastSource<KclRaycastSet>>();
                orbit_cam.1.is_active = false;
                // commands
                //     .entity(topdown_cam.0)
                //     .remove::<RaycastSource<KmpRaycastSet>>()
                //     .remove::<RaycastSource<KclRaycastSet>>();
                topdown_cam.1.is_active = false;
            }
            CameraMode::Orbit => {
                // commands
                //     .entity(fly_cam.0)
                //     .remove::<RaycastSource<KmpRaycastSet>>()
                //     .remove::<RaycastSource<KclRaycastSet>>();
                fly_cam.1.is_active = false;
                // commands
                //     .entity(orbit_cam.0)
                //     .insert(RaycastSource::<KmpRaycastSet>::new())
                //     .insert(RaycastSource::<KclRaycastSet>::new());
                orbit_cam.1.is_active = true;
                // commands
                //     .entity(topdown_cam.0)
                //     .remove::<RaycastSource<KmpRaycastSet>>()
                //     .remove::<RaycastSource<KclRaycastSet>>();
                topdown_cam.1.is_active = false;
            }
            CameraMode::TopDown => {
                // commands
                //     .entity(fly_cam.0)
                //     .remove::<RaycastSource<KmpRaycastSet>>()
                //     .remove::<RaycastSource<KclRaycastSet>>();
                fly_cam.1.is_active = false;
                // commands
                //     .entity(orbit_cam.0)
                //     .remove::<RaycastSource<KmpRaycastSet>>()
                //     .remove::<RaycastSource<KclRaycastSet>>();
                orbit_cam.1.is_active = false;
                // commands
                //     .entity(topdown_cam.0)
                //     .insert(RaycastSource::<KmpRaycastSet>::new())
                //     .insert(RaycastSource::<KclRaycastSet>::new());
                topdown_cam.1.is_active = true;
            }
        }
    }
}
