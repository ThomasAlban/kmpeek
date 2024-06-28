use bevy::{
    prelude::*,
    render::{camera::RenderTarget, view::RenderLayers},
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
        Camera2dBundle {
            camera: Camera {
                // render to the image
                target: RenderTarget::Image(viewport.handle.clone()),
                // render above the main cameras
                order: 1,
                // transparent
                clear_color: ClearColorConfig::None,
                ..default()
            },
            ..default()
        },
        RenderLayers::layer(1),
        Gizmo2dCam,
    ));
}
