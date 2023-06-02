use bevy::prelude::*;
use bevy_egui::EguiPlugin;

mod resources;
mod systems;

pub use resources::AppState;
use systems::*;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin)
            .init_resource::<AppState>()
            .add_system(update_ui);
    }
}
