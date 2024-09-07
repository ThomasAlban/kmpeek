use crate::util::egui_has_primary_context;

use super::{file_dialog::show_file_dialog, menu_bar::show_menu_bar, tabs::show_dock_area};
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use std::path::PathBuf;

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub struct UpdateUiSet;

pub fn update_ui_plugin(app: &mut App) {
    app.add_event::<KmpFileSelected>()
        .add_event::<KclFileSelected>()
        .add_systems(Startup, setup_ui_images)
        .add_systems(Update, update_ui.in_set(UpdateUiSet).run_if(egui_has_primary_context));
}

#[derive(Event, Deref, DerefMut)]
pub struct KmpFileSelected(pub PathBuf);

#[derive(Event, Deref, DerefMut)]
pub struct KclFileSelected(pub PathBuf);

fn setup_ui_images(mut contexts: EguiContexts) {
    egui_extras::install_image_loaders(contexts.ctx_mut());
}

fn update_ui(world: &mut World) {
    show_menu_bar(world);
    show_dock_area(world);
    show_file_dialog(world);
    world.flush();
}
