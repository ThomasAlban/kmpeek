use super::{
    file_dialog::{show_file_dialog, FileDialogParams},
    menu_bar::{show_menu_bar, MenuBarParams},
    tabs::{show_dock_area, DockAreaParams},
};
use bevy::prelude::*;
use std::path::PathBuf;

pub struct UpdateUIPlugin;
impl Plugin for UpdateUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<KmpFileSelected>()
            .add_event::<KclFileSelected>()
            .add_systems(Update, update_ui);
    }
}

#[derive(Event)]
pub struct KmpFileSelected(pub PathBuf);

#[derive(Event)]
pub struct KclFileSelected(pub PathBuf);

fn update_ui(mut p: ParamSet<(MenuBarParams, DockAreaParams, FileDialogParams)>) {
    show_menu_bar(p.p0());
    show_dock_area(p.p1());
    show_file_dialog(p.p2());
}
