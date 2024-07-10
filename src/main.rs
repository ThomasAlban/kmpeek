mod ui;
mod util;
mod viewer;

use bevy::{
    log::{Level, LogPlugin},
    prelude::*,
};
use ui::ui_plugin;
use viewer::viewer_plugin;

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "KMPeek".into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    level: Level::DEBUG,
                    filter: "bevy_ecs=debug".to_string(),
                    custom_layer: |_| None,
                }),
        )
        // .insert_resource(WinitSettings::desktop_app())
        .add_plugins((viewer_plugin, ui_plugin))
        .run();
}
