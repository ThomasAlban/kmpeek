use self::{
    app_state::AppStatePlugin, keybinds::KeybindsPlugin, tabs::DockTreePlugin,
    update_ui::UpdateUIPlugin, viewport::ViewportPlugin,
};
use bevy::app::Plugin;
use bevy_egui::EguiPlugin;

pub mod app_state;
mod file_dialog;
mod keybinds;
mod menu_bar;
mod tabs;
pub mod update_ui;
pub mod viewport;
pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins((
            EguiPlugin,
            AppStatePlugin,
            DockTreePlugin,
            UpdateUIPlugin,
            ViewportPlugin,
            KeybindsPlugin,
        ));
    }
}
