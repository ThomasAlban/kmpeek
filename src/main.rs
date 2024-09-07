mod ui;
mod util;
mod viewer;

use bevy::{prelude::*, winit::WinitSettings};
use ui::ui_plugin;
use viewer::viewer_plugin;

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "KMPeek".into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(WinitSettings::desktop_app())
        .add_plugins((viewer_plugin, ui_plugin))
        .run();
}
