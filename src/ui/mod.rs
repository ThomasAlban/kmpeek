use self::{
    app_state::AppStatePlugin, dock_tree::DockTreePlugin, update_ui::UpdateUIPlugin,
    viewport::ViewportPlugin,
};
use bevy::app::Plugin;
use bevy_egui::EguiPlugin;

pub mod app_state;
pub mod dock_tree;
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
        ));
    }
}
