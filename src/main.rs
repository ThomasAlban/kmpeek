mod ui;
mod util;
mod viewer;

use bevy::{prelude::*, window::WindowCloseRequested, winit::WinitSettings};
use ui::{
    ui_plugin,
    unsaved_changes::{self, DocumentAction},
};
use viewer::viewer_plugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "KMPeek".into(),
                ..default()
            }),
            close_when_requested: false,
            ..default()
        }))
        .insert_resource(WinitSettings::desktop_app())
        .add_plugins((viewer_plugin, ui_plugin))
        .add_systems(Update, request_close)
        .run();
}

fn request_close(world: &mut World) {
    // KMPeek has one primary window, so any native close request means app exit.
    let close_requested = world
        .resource_mut::<Messages<WindowCloseRequested>>()
        .drain()
        .next()
        .is_some();
    if close_requested {
        unsaved_changes::request(world, DocumentAction::Exit);
    }
}
