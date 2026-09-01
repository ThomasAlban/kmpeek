use bevy::{
    prelude::*,
    camera::visibility::RenderLayers,
    camera::RenderTarget,
};

use crate::ui::viewport::{SetupViewportSet, ViewportImage};

pub fn gizmo_2d_cam_plugin(app: &mut App) {
    app.add_systems(Startup, camera_setup.after(SetupViewportSet));
}

// this is a camera for rendering gizmos on top of the 3d scene
#[derive(Component)]
pub struct Gizmo2dCam;

fn camera_setup(mut commands: Commands, viewport: Res<ViewportImage>) {
    commands.spawn((
        Camera2d::default(),
        Camera {
            // render to the image
            target: RenderTarget::Image(viewport.handle.clone().into()),
            // render above the main cameras
            order: 1,
            // transparent
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(1),
        Gizmo2dCam,
        Msaa::Sample4,
    ));
}
