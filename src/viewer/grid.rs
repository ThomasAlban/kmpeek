use bevy::{
    app::{App, Startup},
    ecs::system::Commands,
    prelude::default,
};
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridPlugin, InfiniteGridSettings};

pub fn grid_plugin(app: &mut App) {
    app.add_plugins(InfiniteGridPlugin).add_systems(Startup, setup);
}

fn setup(mut commands: Commands) {
    commands.spawn(InfiniteGridBundle {
        settings: InfiniteGridSettings {
            fadeout_distance: 400000.,
            scale: 0.001,
            ..default()
        },
        ..default()
    });
}
