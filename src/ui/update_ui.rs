use crate::util::egui_has_primary_context;

use super::{file_dialog::show_file_dialog, menu_bar::show_menu_bar, tabs::show_dock_area};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};
use std::path::PathBuf;

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub struct UpdateUiSet;

pub fn update_ui_plugin(app: &mut App) {
    app.add_message::<KmpFileSelected>()
        .add_message::<KclFileSelected>()
        .add_systems(EguiPrimaryContextPass, setup_ui_images.before(UpdateUiSet))
        .add_systems(EguiPrimaryContextPass, update_ui.in_set(UpdateUiSet));
}

#[derive(Message, Deref, DerefMut)]
pub struct KmpFileSelected(pub PathBuf);

#[derive(Message, Deref, DerefMut)]
pub struct KclFileSelected(pub PathBuf);

fn setup_ui_images(mut contexts: EguiContexts) {
    egui_extras::install_image_loaders(contexts.ctx_mut().unwrap());
}

fn update_ui(world: &mut World) {
    println!("Updating UI");
    show_menu_bar(world);
    show_dock_area(world);
    show_file_dialog(world);
    world.flush();
}
