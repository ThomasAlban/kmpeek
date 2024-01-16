use self::{
    keybinds::KeybindsPlugin, settings::AppSettingsPlugin, tabs::DockTreePlugin,
    ui_state::UiStatePlugin, update_ui::UpdateUIPlugin, viewport::ViewportPlugin,
};
use bevy::app::Plugin;
use bevy_egui::EguiPlugin;

mod file_dialog;
mod keybinds;
mod menu_bar;
pub mod settings;
pub mod tabs;
pub mod ui_state;
pub mod update_ui;
mod util;
pub mod viewport;
pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins((
            EguiPlugin,
            UiStatePlugin,
            DockTreePlugin,
            UpdateUIPlugin,
            ViewportPlugin,
            KeybindsPlugin,
            AppSettingsPlugin,
        ));
    }
}
