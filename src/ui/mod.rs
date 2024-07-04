use self::{
    keybinds::keybinds_plugin, settings::app_settings_plugin, tabs::docktree_plugin, ui_state::ui_state_plugin,
    update_ui::update_ui_plugin, viewport::viewport_plugin,
};
use bevy::app::App;
use bevy_egui::EguiPlugin;
use file_dialog::file_dialog_plugin;

pub mod file_dialog;
pub mod keybinds;
mod menu_bar;
pub mod settings;
pub mod tabs;
pub mod ui_state;
pub mod update_ui;
pub mod util;
pub mod viewport;

pub fn ui_plugin(app: &mut App) {
    app.add_plugins((
        EguiPlugin,
        ui_state_plugin,
        docktree_plugin,
        update_ui_plugin,
        viewport_plugin,
        keybinds_plugin,
        app_settings_plugin,
        file_dialog_plugin,
    ));
}
