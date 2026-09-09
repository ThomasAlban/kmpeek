use super::{CameraMode, EditorCamera, UpdateCameraSet};
use crate::ui::{
    settings::AppSettings,
    viewport::{SetupViewportSet, ViewportImage, ViewportInfo},
};
use bevy::{
    camera::RenderTarget,
    input::mouse::{MouseMotion, MouseWheel},
    math::vec3,
    prelude::*,
};
use serde::{Deserialize, Serialize};

pub fn topdown_cam_plugin(app: &mut App) {
    app.add_systems(Startup, camera_setup.after(SetupViewportSet))
        .add_systems(Update, topdown_cam.in_set(UpdateCameraSet));
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
        Camera3d::default(),
        Camera {
            // render to the image
            target: RenderTarget::Image(viewport.handle.clone().into()),
            is_active: false,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            near: topdown_default.near,
            far: topdown_default.far,
            scale: topdown_default.scale,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(topdown_default.start_pos).looking_at(Vec3::ZERO, Vec3::Z),
        TopDownCam,
        EditorCamera,
        Msaa::Sample4,
    ));
}

fn topdown_cam(
    q_window: Query<&mut Window>,
    mut ev_mouse_motion: MessageReader<MouseMotion>,
    mut ev_mouse_scroll: MessageReader<MouseWheel>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut q_topdown_cam: Query<(&mut Transform, &mut Projection), With<TopDownCam>>,
    settings: Res<AppSettings>,
    viewport_info: Res<ViewportInfo>,
) {
    if !viewport_info.mouse_in_viewport || settings.camera.mode != CameraMode::TopDown {
        return;
    }

    let Ok(window) = q_window.single() else { return };

    let mut pan = Vec2::ZERO;
    let mut scroll = 0.;

    if mouse_buttons.pressed(settings.camera.orbit.key_bindings.mouse_button) {
        for ev in ev_mouse_motion.read() {
            pan += ev.delta;
        }
    }
    for ev in ev_mouse_scroll.read() {
        scroll += ev.y;
    }

    let window_size = Vec2::new(window.width(), window.height());

    let Ok((mut transform, mut projection)) = q_topdown_cam.single_mut() else {
        return;
    };
    let mut transform_cp = *transform;

    if let Projection::Orthographic(projection) = &*projection {
        pan *= Vec2::new(projection.area.width(), projection.area.height()) / window_size;
    }
    transform_cp.translation += vec3(pan.x, 0., pan.y) * settings.camera.top_down.move_sensitivity;

    if scroll.abs() > 0. {
        if let Projection::Orthographic(projection) = &mut *projection {
            projection.scale -= (scroll * projection.scale) * 0.001 * settings.camera.top_down.scroll_sensitivity;
            projection.scale = projection.scale.clamp(1., 500.);
        }
    }

    transform.set_if_neq(transform_cp);

    ev_mouse_motion.clear();
}
