mod ui;
mod util;
mod viewer;

use bevy::{prelude::*, window::PresentMode, winit::WinitSettings};
use ui::UIPlugin;
use viewer::ViewerPlugin;

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "KMPeek".into(),
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(WinitSettings::desktop_app())
        .add_plugins((ViewerPlugin, UIPlugin))
        .run();
}
