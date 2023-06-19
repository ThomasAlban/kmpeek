use bevy::prelude::*;
use bevy_egui::EguiPlugin;

mod resources;
mod systems;

pub use resources::AppState;
pub use systems::FileSelected;
use systems::*;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin)
            .init_resource::<AppState>()
            .add_event::<FileSelected>()
            .add_system(update_ui)
            .add_system(file_dialogue);
    }
}
