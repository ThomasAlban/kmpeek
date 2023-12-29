use super::CameraMode;
use crate::ui::{
    app_state::{AppSettings, AppState},
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

pub struct TopDownCamPlugin;
impl Plugin for TopDownCamPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, camera_setup.after(SetupViewportSet))
            .add_systems(Update, topdown_cam);
    }
}

#[derive(Component)]
pub struct TopDownCam;

#[derive(Serialize, Deserialize)]
pub struct TopDownSettings {
    pub start_pos: Vec3,
    pub near: f32,
    pub far: f32,
    pub scale: f32,
    pub move_sensitivity: f32,
    pub scroll_sensitivity: f32,
    pub key_bindings: TopDownKeyBindings,
}
impl Default for TopDownSettings {
    fn default() -> Self {
        Self {
            start_pos: vec3(0., 100000., 0.),
            near: 0.000001,
            far: 1000000.,
            scale: 100.,
            move_sensitivity: 1.,
            scroll_sensitivity: 1.,
            key_bindings: TopDownKeyBindings::default(),
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct TopDownKeyBindings {
    pub mouse_button: MouseButton,
}
impl Default for TopDownKeyBindings {
    fn default() -> Self {
        Self {
            mouse_button: MouseButton::Right,
        }
    }
}

fn camera_setup(mut commands: Commands, viewport: Res<ViewportImage>) {
    let topdown_default = TopDownSettings::default();

    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                // render to the image
                target: RenderTarget::Image(viewport.clone()),
                is_active: false,
                ..default()
            },
            projection: Projection::Orthographic(OrthographicProjection {
                near: topdown_default.near,
                far: topdown_default.far,
                scale: topdown_default.scale,
                ..default()
            }),
            transform: Transform::from_translation(topdown_default.start_pos)
                .looking_at(Vec3::ZERO, Vec3::Z),
            ..default()
        },
        TopDownCam,
    ));
}

fn topdown_cam(
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_scroll: EventReader<MouseWheel>,
    mouse_buttons: Res<Input<MouseButton>>,
    mut query: Query<(&TopDownCam, &mut Transform, &mut Projection)>,
    app_state: Res<AppState>,
    settings: Res<AppSettings>,
) {
    if !app_state.mouse_in_viewport || settings.camera.mode != CameraMode::TopDown {
        return;
    }

    let window = primary_window.get_single().unwrap();

    let mut pan = Vec2::ZERO;
    let mut scroll = 0.;

    if mouse_buttons.pressed(settings.camera.orbit.key_bindings.mouse_button) {
        for ev in mouse_motion.read() {
            pan += ev.delta;
        }
    }
    for ev in mouse_scroll.read() {
        scroll += ev.y;
    }

    let window_size = Vec2::new(window.width(), window.height());

    for (_, mut transform, mut projection) in query.iter_mut() {
        if let Projection::Orthographic(projection) = &*projection {
            pan *= Vec2::new(projection.area.width(), projection.area.height()) / window_size;
        }
        transform.translation += vec3(pan.x, 0., pan.y) * settings.camera.top_down.move_sensitivity;

        if scroll.abs() > 0. {
            if let Projection::Orthographic(projection) = &mut *projection {
                projection.scale -= (scroll * projection.scale)
                    * 0.001
                    * settings.camera.top_down.scroll_sensitivity;
                projection.scale = projection.scale.clamp(1., 1000.);
            }
        }
    }
    mouse_motion.clear();
}
