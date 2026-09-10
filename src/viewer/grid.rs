use bevy::{
    app::{App, Startup},
    dev_tools::infinite_grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    ecs::system::Commands,
    prelude::Color,
};

pub fn grid_plugin(app: &mut App) {
    app.add_plugins(InfiniteGridPlugin).add_systems(Startup, setup);
}

fn setup(mut commands: Commands) {
    commands.spawn((
        InfiniteGrid,
        InfiniteGridSettings {
            x_axis_color: Color::srgb(1.0, 0.0, 0.0),
            z_axis_color: Color::srgb(0.0, 0.0, 1.0),
            minor_line_color: Color::srgb(0.2, 0.2, 0.2),
            major_line_color: Color::srgb(0.25, 0.25, 0.25),
            fadeout_distance: 400000.,
            dot_fadeout_strength: 0.25,
            scale: 0.001,
        },
    ));
}
