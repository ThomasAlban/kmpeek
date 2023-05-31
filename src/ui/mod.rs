use bevy::prelude::*;
use bevy_egui::EguiPlugin;

mod resources;
mod systems;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin)
            .init_resource::<resources::UIOptions>()
            .add_system(systems::update_ui);
    }
}
